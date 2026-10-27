//! `keepers-fixtures`: writes the generated test fixtures (`docs/TESTING.md` §4).
//!
//! Every byte is reproducible: pixels come from integer maths, the few transcendental values
//! (Gaussian taps, the sRGB curve, the exposure factor) from `libm` quantised to fixed point,
//! JPEGs from the `image` encoder at a fixed quality, and EXIF from the in-house writer. A
//! committed SHA-256 manifest proves the output is identical on Windows and Linux.

mod blur;
mod exif_writer;
mod scene;

use std::fs;
use std::io::{self, Cursor, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use image::RgbImage;
use image::codecs::jpeg::JpegEncoder;
use sha2::{Digest, Sha256};

use exif_writer::ExifFields;
use scene::Rng;

const SCENE_WIDTH: u32 = 1536;
const SCENE_HEIGHT: u32 = 1024;
const JPEG_QUALITY: u8 = 95;

#[derive(Parser, Debug)]
#[command(
    name = "keepers-fixtures",
    about = "Writes the seeded test fixtures of Keepers, bit-identical on every platform."
)]
struct Args {
    /// The directory to write the fixture sets into (created if missing)
    #[arg(long)]
    out: PathBuf,

    /// The seed every set derives its own stream from
    #[arg(long, default_value_t = 1)]
    seed: u64,

    /// After writing, compare every file against this SHA-256 manifest
    #[arg(long)]
    check: Option<PathBuf>,

    /// After writing, record the SHA-256 of every file in this manifest
    #[arg(long)]
    write_manifest: Option<PathBuf>,
}

fn main() -> ExitCode {
    let args = Args::parse();
    if let Err(e) = generate(&args.out, args.seed) {
        eprintln!("keepers-fixtures: could not write the fixtures: {e}");
        return ExitCode::from(2);
    }
    if let Some(path) = &args.write_manifest {
        if let Err(e) = write_manifest(&args.out, path) {
            eprintln!("keepers-fixtures: could not write the manifest: {e}");
            return ExitCode::from(2);
        }
        println!("manifest written: {}", path.display());
    }
    if let Some(path) = &args.check {
        match check_manifest(&args.out, path) {
            Ok(count) => println!("fixtures match the manifest ({count} files)"),
            Err(message) => {
                eprintln!("{message}");
                return ExitCode::from(1);
            }
        }
    }
    ExitCode::SUCCESS
}

/// Writes every set. Each set derives its own random stream from the seed and its name, so
/// adding a set in a later module never changes the bytes of an existing one.
fn generate(out: &Path, seed: u64) -> io::Result<()> {
    fs::create_dir_all(out)?;
    let written = write_basic(&out.join("basic"), seed)?;
    println!("fixtures: basic ({written} files)");
    Ok(())
}

/// The `basic` set: the base scene, the same scene blurred with σ 2, and the same scene one
/// stop brighter in linear light. Proves the generator and the harness work.
fn write_basic(dir: &Path, seed: u64) -> io::Result<usize> {
    fs::create_dir_all(dir)?;
    let mut rng = Rng::new(scene::set_seed(seed, "basic"));
    let base = scene::base_scene(SCENE_WIDTH, SCENE_HEIGHT, &mut rng);
    let blurred = blur::gaussian_blur(&base, 2_000);
    let brighter = scene::exposure_shift(&base, 1_000);

    let frames: [(&str, &RgbImage, &str, &str, &str); 3] = [
        ("IMG_0001.jpg", &base, "base", "0.0", "0.0"),
        ("IMG_0002.jpg", &blurred, "blur", "2.0", "0.0"),
        ("IMG_0003.jpg", &brighter, "exposure", "0.0", "1.0"),
    ];
    let mut truth = String::new();
    truth.push_str(
        "# Facts the basic set was built from. Expected results come from these, never from\n\
         # running Keepers. The seed and integer maths make every byte reproducible.\n",
    );
    truth.push_str(&format!(
        "schema = 1\nset = \"basic\"\nseed = {seed}\nwidth = {SCENE_WIDTH}\nheight = {SCENE_HEIGHT}\n\
         jpeg_quality = {JPEG_QUALITY}\n\n[exif]\nmake = \"Keepers Fixtures\"\nmodel = \"Generator 1\"\n\
         orientation = 1\nsub_sec_time_original = \"340\"\noffset_time_original = \"+01:00\"\n"
    ));
    for (i, (file, img, variant, sigma, ev)) in frames.iter().enumerate() {
        let date_time_original = format!("2026:06:13 15:02:{:02}", 11 + i);
        let exif = ExifFields {
            make: "Keepers Fixtures",
            model: "Generator 1",
            orientation: 1,
            date_time_original: &date_time_original,
            sub_sec_time_original: "340",
            offset_time_original: "+01:00",
        };
        let jpeg = encode_jpeg(img)?;
        let with_exif = exif_writer::insert_app1(&jpeg, &exif);
        fs::write(dir.join(file), with_exif)?;
        truth.push_str(&format!(
            "\n[[frame]]\nfile = \"{file}\"\nvariant = \"{variant}\"\nblur_sigma = {sigma}\nev = {ev}\n\
             date_time_original = \"{date_time_original}\"\n"
        ));
    }
    fs::write(dir.join("truth.toml"), truth)?;
    Ok(frames.len() + 1)
}

fn encode_jpeg(img: &RgbImage) -> io::Result<Vec<u8>> {
    let mut buffer = Cursor::new(Vec::new());
    let mut encoder = JpegEncoder::new_with_quality(&mut buffer, JPEG_QUALITY);
    encoder.encode_image(img).map_err(io::Error::other)?;
    Ok(buffer.into_inner())
}

/// Every file under `dir`, as a sorted list of `/`-separated paths relative to `dir`.
fn relative_files(dir: &Path) -> io::Result<Vec<String>> {
    fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) -> io::Result<()> {
        let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<io::Result<_>>()?;
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            let relative = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            if entry.file_type()?.is_dir() {
                walk(&entry.path(), &relative, out)?;
            } else {
                out.push(relative);
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    walk(dir, "", &mut files)?;
    files.sort();
    Ok(files)
}

fn sha256_hex(path: &Path) -> io::Result<String> {
    let bytes = fs::read(path)?;
    let digest = Sha256::digest(&bytes);
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}

/// Writes `<sha256>  <relative path>` per line, sorted by path, LF line endings.
fn write_manifest(dir: &Path, manifest: &Path) -> io::Result<()> {
    let mut out = Vec::new();
    for relative in relative_files(dir)? {
        writeln!(out, "{}  {relative}", sha256_hex(&dir.join(&relative))?)?;
    }
    fs::write(manifest, out)
}

/// Compares every file under `dir` with the manifest. The first mismatch is the error.
fn check_manifest(dir: &Path, manifest: &Path) -> Result<usize, String> {
    let text = fs::read_to_string(manifest)
        .map_err(|e| format!("cannot read the manifest {}: {e}", manifest.display()))?;
    let mut listed = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let (hash, relative) = line
            .split_once("  ")
            .ok_or_else(|| format!("manifest line {} is not '<sha256>  <path>'", n + 1))?;
        listed.push((hash.to_string(), relative.to_string()));
    }
    for (hash, relative) in &listed {
        let actual = sha256_hex(&dir.join(relative)).ok();
        if actual.as_deref() != Some(hash.as_str()) {
            return Err(format!("fixture {relative} differs from the manifest"));
        }
    }
    let present = relative_files(dir).map_err(|e| format!("cannot list the fixtures: {e}"))?;
    for relative in present {
        if !listed.iter().any(|(_, listed)| *listed == relative) {
            return Err(format!("fixture {relative} is not in the manifest"));
        }
    }
    Ok(listed.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_round_trips_and_detects_a_change() {
        let tmp =
            std::env::temp_dir().join(format!("keepers-fixtures-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let out = tmp.join("gen");
        fs::create_dir_all(out.join("set")).unwrap();
        fs::write(out.join("set").join("a.txt"), b"alpha").unwrap();
        fs::write(out.join("b.txt"), b"beta").unwrap();
        let manifest = tmp.join("gen.sha256");
        write_manifest(&out, &manifest).unwrap();
        let text = fs::read_to_string(&manifest).unwrap();
        assert_eq!(text.lines().count(), 2);
        assert!(
            text.lines()
                .all(|l| l.len() == 64 + 2 + l.split("  ").nth(1).unwrap().len())
        );
        assert!(text.starts_with(&format!(
            "{}  b.txt\n",
            sha256_hex(&out.join("b.txt")).unwrap()
        )));
        assert_eq!(check_manifest(&out, &manifest), Ok(2));

        fs::write(out.join("b.txt"), b"changed").unwrap();
        assert_eq!(
            check_manifest(&out, &manifest),
            Err("fixture b.txt differs from the manifest".to_string())
        );
        fs::write(out.join("b.txt"), b"beta").unwrap();
        fs::write(out.join("extra.txt"), b"extra").unwrap();
        assert_eq!(
            check_manifest(&out, &manifest),
            Err("fixture extra.txt is not in the manifest".to_string())
        );
        fs::remove_file(out.join("extra.txt")).unwrap();
        fs::remove_file(out.join("set").join("a.txt")).unwrap();
        assert_eq!(
            check_manifest(&out, &manifest),
            Err("fixture set/a.txt differs from the manifest".to_string())
        );
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn sha256_of_a_known_string() {
        let tmp = std::env::temp_dir().join(format!("keepers-fixtures-sha-{}", std::process::id()));
        fs::write(&tmp, b"abc").unwrap();
        // SHA-256("abc") from FIPS 180-4, example B.1.
        assert_eq!(
            sha256_hex(&tmp).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let _ = fs::remove_file(&tmp);
    }
}
