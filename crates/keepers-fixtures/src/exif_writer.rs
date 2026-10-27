//! A minimal EXIF writer: one APP1 segment holding a big-endian TIFF header, IFD0 and the Exif
//! IFD, with entries sorted by tag and values on word boundaries.
//!
//! References: TIFF 6.0 (Adobe, 1992) §2 for the header, the IFD layout, the field types and the
//! rule that IFDs and values begin on a word boundary; EXIF 2.32 (CIPA DC-008-2019) §4.6 for the
//! `Exif\0\0` identifier, the tags and the ASCII date format.

const TYPE_ASCII: u16 = 2;
const TYPE_SHORT: u16 = 3;
const TYPE_LONG: u16 = 4;

const TAG_MAKE: u16 = 0x010F;
const TAG_MODEL: u16 = 0x0110;
const TAG_ORIENTATION: u16 = 0x0112;
const TAG_EXIF_IFD: u16 = 0x8769;
const TAG_DATE_TIME_ORIGINAL: u16 = 0x9003;
const TAG_OFFSET_TIME_ORIGINAL: u16 = 0x9011;
const TAG_SUB_SEC_TIME_ORIGINAL: u16 = 0x9291;

/// The fields the fixtures carry.
pub struct ExifFields<'a> {
    pub make: &'a str,
    pub model: &'a str,
    pub orientation: u16,
    /// `YYYY:MM:DD HH:MM:SS`
    pub date_time_original: &'a str,
    pub sub_sec_time_original: &'a str,
    /// `+HH:MM` or `-HH:MM`
    pub offset_time_original: &'a str,
}

#[derive(Clone)]
struct Entry {
    tag: u16,
    kind: u16,
    count: u32,
    data: Vec<u8>,
}

fn ascii(tag: u16, text: &str) -> Entry {
    let mut data = text.as_bytes().to_vec();
    data.push(0);
    Entry {
        tag,
        kind: TYPE_ASCII,
        count: data.len() as u32,
        data,
    }
}

fn short(tag: u16, value: u16) -> Entry {
    Entry {
        tag,
        kind: TYPE_SHORT,
        count: 1,
        data: value.to_be_bytes().to_vec(),
    }
}

fn long(tag: u16, value: u32) -> Entry {
    Entry {
        tag,
        kind: TYPE_LONG,
        count: 1,
        data: value.to_be_bytes().to_vec(),
    }
}

/// Appends one IFD at the current end of `out` (which must be even), with its entries sorted by
/// tag. Values of four bytes or fewer sit in the entry, left-justified; longer values follow the
/// IFD in tag order, each starting on an even offset.
fn append_ifd(out: &mut Vec<u8>, entries: &[Entry]) {
    debug_assert!(
        out.len().is_multiple_of(2),
        "an IFD begins on a word boundary"
    );
    let mut entries = entries.to_vec();
    entries.sort_by_key(|e| e.tag);
    let mut next_value = (out.len() + 2 + 12 * entries.len() + 4) as u32;
    let mut deferred: Vec<(u32, Vec<u8>)> = Vec::new();

    out.extend_from_slice(&(entries.len() as u16).to_be_bytes());
    for entry in &entries {
        out.extend_from_slice(&entry.tag.to_be_bytes());
        out.extend_from_slice(&entry.kind.to_be_bytes());
        out.extend_from_slice(&entry.count.to_be_bytes());
        if entry.data.len() <= 4 {
            let mut inline = [0u8; 4];
            inline[..entry.data.len()].copy_from_slice(&entry.data);
            out.extend_from_slice(&inline);
        } else {
            if next_value % 2 == 1 {
                next_value += 1;
            }
            out.extend_from_slice(&next_value.to_be_bytes());
            deferred.push((next_value, entry.data.clone()));
            next_value += entry.data.len() as u32;
        }
    }
    out.extend_from_slice(&0u32.to_be_bytes());
    for (offset, data) in deferred {
        while out.len() < offset as usize {
            out.push(0);
        }
        out.extend_from_slice(&data);
    }
}

/// The TIFF structure: header, IFD0 (Make, Model, Orientation, Exif IFD pointer) and the Exif
/// IFD (DateTimeOriginal, OffsetTimeOriginal, SubSecTimeOriginal).
pub fn tiff_bytes(fields: &ExifFields<'_>) -> Vec<u8> {
    let header = [b'M', b'M', 0x00, 0x2A, 0x00, 0x00, 0x00, 0x08];
    let ifd0_with = |exif_offset: u32| {
        vec![
            ascii(TAG_MAKE, fields.make),
            ascii(TAG_MODEL, fields.model),
            short(TAG_ORIENTATION, fields.orientation),
            long(TAG_EXIF_IFD, exif_offset),
        ]
    };
    // The Exif IFD follows IFD0 and its values, on the next word boundary; a first pass with a
    // placeholder pointer measures where that is.
    let mut probe = header.to_vec();
    append_ifd(&mut probe, &ifd0_with(0));
    let exif_offset = (probe.len() as u32).next_multiple_of(2);

    let mut out = header.to_vec();
    append_ifd(&mut out, &ifd0_with(exif_offset));
    while out.len() < exif_offset as usize {
        out.push(0);
    }
    append_ifd(
        &mut out,
        &[
            ascii(TAG_DATE_TIME_ORIGINAL, fields.date_time_original),
            ascii(TAG_OFFSET_TIME_ORIGINAL, fields.offset_time_original),
            ascii(TAG_SUB_SEC_TIME_ORIGINAL, fields.sub_sec_time_original),
        ],
    );
    out
}

/// The APP1 payload: `Exif\0\0` followed by the TIFF structure (everything after the segment
/// length).
pub fn app1_payload(fields: &ExifFields<'_>) -> Vec<u8> {
    let mut out = b"Exif\0\0".to_vec();
    out.extend(tiff_bytes(fields));
    out
}

/// The whole APP1 segment: marker, length (which counts itself) and payload.
pub fn app1_segment(fields: &ExifFields<'_>) -> Vec<u8> {
    let payload = app1_payload(fields);
    let length = (payload.len() + 2) as u16;
    let mut out = vec![0xFF, 0xE1];
    out.extend_from_slice(&length.to_be_bytes());
    out.extend(payload);
    out
}

/// Inserts the APP1 segment immediately after the SOI marker, where EXIF 2.32 §4.5.4 puts it.
pub fn insert_app1(jpeg: &[u8], fields: &ExifFields<'_>) -> Vec<u8> {
    assert!(jpeg.starts_with(&[0xFF, 0xD8]), "not a JPEG: no SOI marker");
    let mut out = vec![0xFF, 0xD8];
    out.extend(app1_segment(fields));
    out.extend_from_slice(&jpeg[2..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ExifFields<'static> {
        ExifFields {
            make: "Keepers Fixtures",
            model: "Generator 1",
            orientation: 1,
            date_time_original: "2026:06:13 15:02:11",
            sub_sec_time_original: "340",
            offset_time_original: "+01:00",
        }
    }

    /// Worked out by hand from TIFF 6.0 and EXIF 2.32: big-endian header with IFD0 at 8; IFD0
    /// with four entries in tag order (Make at 62, Model at 80 after a pad byte, Orientation
    /// inline, the Exif IFD pointer to 92); the Exif IFD with three entries in tag order
    /// (DateTimeOriginal at 134, OffsetTimeOriginal at 154, SubSecTimeOriginal inline).
    const EXPECTED_PAYLOAD: [u8; 167] = [
        0x45, 0x78, 0x69, 0x66, 0x00, 0x00, // "Exif\0\0"
        0x4D, 0x4D, 0x00, 0x2A, 0x00, 0x00, 0x00, 0x08, // TIFF header, IFD0 at 8
        0x00, 0x04, // IFD0: 4 entries
        0x01, 0x0F, 0x00, 0x02, 0x00, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00, 0x3E, // Make
        0x01, 0x10, 0x00, 0x02, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00, 0x50, // Model
        0x01, 0x12, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, // Orientation
        0x87, 0x69, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x5C, // Exif IFD
        0x00, 0x00, 0x00, 0x00, // no next IFD
        0x4B, 0x65, 0x65, 0x70, 0x65, 0x72, 0x73, 0x20, 0x46, 0x69, 0x78, 0x74, 0x75, 0x72, 0x65,
        0x73, 0x00, // "Keepers Fixtures\0" at 62
        0x00, // pad to a word boundary
        0x47, 0x65, 0x6E, 0x65, 0x72, 0x61, 0x74, 0x6F, 0x72, 0x20, 0x31,
        0x00, // "Generator 1\0" at 80
        0x00, 0x03, // Exif IFD at 92: 3 entries
        0x90, 0x03, 0x00, 0x02, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00,
        0x86, // DateTimeOriginal
        0x90, 0x11, 0x00, 0x02, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00,
        0x9A, // OffsetTimeOriginal
        0x92, 0x91, 0x00, 0x02, 0x00, 0x00, 0x00, 0x04, 0x33, 0x34, 0x30,
        0x00, // SubSecTimeOriginal
        0x00, 0x00, 0x00, 0x00, // no next IFD
        0x32, 0x30, 0x32, 0x36, 0x3A, 0x30, 0x36, 0x3A, 0x31, 0x33, 0x20, 0x31, 0x35, 0x3A, 0x30,
        0x32, 0x3A, 0x31, 0x31, 0x00, // "2026:06:13 15:02:11\0" at 134
        0x2B, 0x30, 0x31, 0x3A, 0x30, 0x30, 0x00, // "+01:00\0" at 154
    ];

    #[test]
    fn app1_payload_matches_the_bytes_worked_out_by_hand() {
        let payload = app1_payload(&sample());
        assert_eq!(payload.len(), EXPECTED_PAYLOAD.len());
        for (i, (got, want)) in payload.iter().zip(EXPECTED_PAYLOAD.iter()).enumerate() {
            assert_eq!(got, want, "byte {i} differs");
        }
    }

    #[test]
    fn segment_length_counts_itself_and_sits_after_soi() {
        let segment = app1_segment(&sample());
        assert_eq!(&segment[..2], &[0xFF, 0xE1]);
        assert_eq!(u16::from_be_bytes([segment[2], segment[3]]), 169);
        assert_eq!(segment.len(), 171);

        let jpeg = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x02, 0xFF, 0xD9];
        let with_exif = insert_app1(&jpeg, &sample());
        assert_eq!(&with_exif[..2], &[0xFF, 0xD8]);
        assert_eq!(&with_exif[2..173], &segment[..]);
        assert_eq!(&with_exif[173..], &jpeg[2..]);
    }

    #[test]
    fn entries_are_sorted_by_tag_whatever_the_input_order() {
        let mut out = Vec::new();
        append_ifd(
            &mut out,
            &[short(0x0112, 1), ascii(0x010F, "a"), long(0x8769, 8)],
        );
        let tags: Vec<u16> = (0..3)
            .map(|i| u16::from_be_bytes([out[2 + 12 * i], out[3 + 12 * i]]))
            .collect();
        assert_eq!(tags, vec![0x010F, 0x0112, 0x8769]);
    }
}
