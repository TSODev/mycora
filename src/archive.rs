use std::fs::File;
use std::path::Path;

use anyhow::{bail, Context, Result};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;

/// Compresses every file under `vault_dir` into a single gzip-compressed
/// tar at `archive_path`. Paths inside the archive are relative to
/// `vault_dir` itself (not prefixed with its own directory name), so
/// `unarchive_vault_dir` can extract straight back into a directory of
/// the same shape regardless of what that directory happens to be named
/// on either end of the round trip.
pub fn archive_vault_dir(vault_dir: &Path, archive_path: &Path) -> Result<()> {
    let file = File::create(archive_path)
        .with_context(|| format!("creating {}", archive_path.display()))?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = tar::Builder::new(encoder);
    builder
        .append_dir_all(".", vault_dir)
        .with_context(|| format!("archiving {}", vault_dir.display()))?;
    builder.finish().context("finalizing archive")?;
    Ok(())
}

/// Sanity-checks that `archive_path` is a readable, non-empty archive
/// before the caller deletes the original directory it was made from —
/// reads every entry's header (doesn't extract anything) and counts the
/// regular files among them (`append_dir_all`'s own "." directory entry
/// doesn't count, so an empty vault is correctly flagged as having zero
/// files rather than the one always-present root entry masking that). A
/// corrupt or truncated archive fails here rather than silently being
/// "verified" and only discovered broken once the original it was meant
/// to replace is already gone.
pub fn verify_archive(archive_path: &Path) -> Result<()> {
    let file = File::open(archive_path)
        .with_context(|| format!("opening {}", archive_path.display()))?;
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    let mut file_count = 0;
    for entry in archive.entries().context("reading archive entries")? {
        let entry = entry.context("reading archive entry")?;
        if entry.header().entry_type().is_file() {
            file_count += 1;
        }
    }
    if file_count == 0 {
        bail!("{} contains no files", archive_path.display());
    }
    Ok(())
}

/// Extracts `archive_path` (as written by `archive_vault_dir`) into
/// `vault_dir`, creating it first if it doesn't already exist.
pub fn unarchive_vault_dir(archive_path: &Path, vault_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(vault_dir)
        .with_context(|| format!("creating {}", vault_dir.display()))?;
    let file = File::open(archive_path)
        .with_context(|| format!("opening {}", archive_path.display()))?;
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(vault_dir)
        .with_context(|| format!("extracting into {}", vault_dir.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("mycora-archive-test-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn round_trips_a_directory_of_files() {
        let base = scratch_dir();
        let vault_dir = base.join("vault");
        std::fs::create_dir_all(vault_dir.join("nested")).unwrap();
        std::fs::write(vault_dir.join("a.md"), "# A\n\nHello.").unwrap();
        std::fs::write(vault_dir.join("nested/b.md"), "# B\n\nWorld.").unwrap();

        let archive_path = base.join("vault.tar.gz");
        archive_vault_dir(&vault_dir, &archive_path).unwrap();
        verify_archive(&archive_path).unwrap();

        let restored_dir = base.join("restored");
        unarchive_vault_dir(&archive_path, &restored_dir).unwrap();

        assert_eq!(
            std::fs::read_to_string(restored_dir.join("a.md")).unwrap(),
            "# A\n\nHello."
        );
        assert_eq!(
            std::fs::read_to_string(restored_dir.join("nested/b.md")).unwrap(),
            "# B\n\nWorld."
        );

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn verify_rejects_an_empty_archive() {
        let base = scratch_dir();
        std::fs::create_dir_all(&base).unwrap();
        let empty_dir = base.join("empty-vault");
        std::fs::create_dir_all(&empty_dir).unwrap();

        let archive_path = base.join("empty.tar.gz");
        archive_vault_dir(&empty_dir, &archive_path).unwrap();

        assert!(verify_archive(&archive_path).is_err());

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn verify_rejects_a_non_gzip_file() {
        let base = scratch_dir();
        std::fs::create_dir_all(&base).unwrap();
        let not_an_archive = base.join("not-an-archive.tar.gz");
        std::fs::write(&not_an_archive, "just some text, not gzip at all").unwrap();

        assert!(verify_archive(&not_an_archive).is_err());

        std::fs::remove_dir_all(&base).ok();
    }
}
