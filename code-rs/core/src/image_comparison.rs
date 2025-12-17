use image::DynamicImage;
use image::ImageReader;
use image_hasher::HashAlg;
use image_hasher::HasherConfig;
use image_hasher::ImageHash;
use std::path::Path;

fn phash_256(img: &DynamicImage) -> ImageHash<[u8; 32]> {
    // "Classic" pHash ≈ Mean + DCT, larger hash for sensitivity
    HasherConfig::with_bytes_type::<[u8; 32]>()
        .hash_size(16, 16)
        .hash_alg(HashAlg::Mean)
        .preproc_dct()
        .to_hasher()
        .hash_image(img)
}

fn dhash_256(img: &DynamicImage) -> ImageHash<[u8; 32]> {
    // Gradient (dHash); good at catching small edge changes
    HasherConfig::with_bytes_type::<[u8; 32]>()
        .hash_size(16, 16)
        .hash_alg(HashAlg::Gradient)
        .to_hasher()
        .hash_image(img)
}

/// Compute a hash for an image that can be stored and compared later
pub fn compute_image_hash<P: AsRef<Path>>(path: P) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    let img = ImageReader::open(path)?.decode()?;
    let phash = phash_256(&img);
    let dhash = dhash_256(&img);

    Ok((phash.as_bytes().to_vec(), dhash.as_bytes().to_vec()))
}

/// Compare image hashes to determine if images are similar
pub fn are_hashes_similar(phash1: &[u8], dhash1: &[u8], phash2: &[u8], dhash2: &[u8]) -> bool {
    if phash1.len() != 32 || dhash1.len() != 32 || phash2.len() != 32 || dhash2.len() != 32 {
        return false;
    }

    // Count differing bits (Hamming distance)
    let phash_dist = phash1
        .iter()
        .zip(phash2.iter())
        .map(|(a, b)| (a ^ b).count_ones() as i32)
        .sum::<i32>();

    let dhash_dist = dhash1
        .iter()
        .zip(dhash2.iter())
        .map(|(a, b)| (a ^ b).count_ones() as i32)
        .sum::<i32>();

    // 256 bits → ~5% tolerance (≈13 bits)
    phash_dist <= 13 && dhash_dist <= 13
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    const ONE_BY_ONE_PNG: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 10, 73, 68, 65, 84, 120, 156, 99, 0, 1, 0, 0, 5, 0, 1,
        13, 10, 45, 180, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];

    #[test]
    fn compute_image_hash_returns_32_byte_hashes_for_png() -> anyhow::Result<()> {
        let file = tempfile::Builder::new().suffix(".png").tempfile()?;
        std::fs::write(file.path(), ONE_BY_ONE_PNG)?;

        let (phash, dhash) = compute_image_hash(file.path())?;
        assert_eq!(phash.len(), 32);
        assert_eq!(dhash.len(), 32);
        Ok(())
    }

    #[test]
    fn are_hashes_similar_rejects_wrong_lengths() {
        assert!(!are_hashes_similar(&[], &[], &[], &[]));
        assert!(!are_hashes_similar(&[0; 32], &[], &[0; 32], &[0; 32]));
    }

    #[test]
    fn are_hashes_similar_uses_13_bit_threshold() {
        let base = vec![0u8; 32];

        let mut diff_13 = base.clone();
        diff_13[0] = 0xFF;
        diff_13[1] = 0x1F;
        assert!(are_hashes_similar(&base, &base, &diff_13, &base));

        let mut diff_14 = base.clone();
        diff_14[0] = 0xFF;
        diff_14[1] = 0x3F;
        assert!(!are_hashes_similar(&base, &base, &diff_14, &base));
    }
}
