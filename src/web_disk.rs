use anyhow::{Context, Result};
use std::{ffi::CString, mem::MaybeUninit, os::unix::ffi::OsStrExt, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiskUsageLevel {
    Normal,
    Warning,
    Critical,
}

impl DiskUsageLevel {
    pub(crate) fn css_class(self) -> &'static str {
        match self {
            Self::Normal => "ok",
            Self::Warning => "warn",
            Self::Critical => "bad",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Normal => "ausreichend",
            Self::Warning => "beobachten",
            Self::Critical => "kritisch",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DiskUsage {
    pub(crate) total_bytes: u64,
    pub(crate) used_bytes: u64,
    pub(crate) available_bytes: u64,
    pub(crate) used_percent: f64,
    pub(crate) level: DiskUsageLevel,
}

pub(crate) fn disk_usage(path: &Path) -> Result<DiskUsage> {
    let path_bytes = path.as_os_str().as_bytes();

    let c_path = CString::new(path_bytes).with_context(|| {
        format!(
            "Pfad enthält ein ungültiges Nullzeichen: {}",
            path.display()
        )
    })?;

    let mut stats = MaybeUninit::<libc::statvfs>::uninit();

    let result = unsafe { libc::statvfs(c_path.as_ptr(), stats.as_mut_ptr()) };

    if result != 0 {
        return Err(std::io::Error::last_os_error()).with_context(|| {
            format!(
                "Speicherplatz konnte nicht gelesen werden: {}",
                path.display()
            )
        });
    }

    let stats = unsafe { stats.assume_init() };

    let block_size = if stats.f_frsize > 0 {
        stats.f_frsize
    } else {
        stats.f_bsize
    };

    let total_bytes = multiply_to_u64(stats.f_blocks as u128, block_size as u128);

    let available_bytes = multiply_to_u64(stats.f_bavail as u128, block_size as u128);

    let used_bytes = total_bytes.saturating_sub(available_bytes);

    let used_percent = if total_bytes == 0 {
        0.0
    } else {
        used_bytes as f64 / total_bytes as f64 * 100.0
    };

    Ok(DiskUsage {
        total_bytes,
        used_bytes,
        available_bytes,
        used_percent,
        level: usage_level(used_percent),
    })
}

pub(crate) fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    const TIB: f64 = GIB * 1024.0;
    const PIB: f64 = TIB * 1024.0;

    let bytes_f = bytes as f64;

    let (value, unit) = if bytes_f >= PIB {
        (bytes_f / PIB, "PiB")
    } else if bytes_f >= TIB {
        (bytes_f / TIB, "TiB")
    } else if bytes_f >= GIB {
        (bytes_f / GIB, "GiB")
    } else if bytes_f >= MIB {
        (bytes_f / MIB, "MiB")
    } else if bytes_f >= KIB {
        (bytes_f / KIB, "KiB")
    } else {
        return format!("{bytes} B");
    };

    format!("{value:.1} {unit}").replace('.', ",")
}

fn usage_level(used_percent: f64) -> DiskUsageLevel {
    if used_percent >= 90.0 {
        DiskUsageLevel::Critical
    } else if used_percent >= 80.0 {
        DiskUsageLevel::Warning
    } else {
        DiskUsageLevel::Normal
    }
}

fn multiply_to_u64(left: u128, right: u128) -> u64 {
    left.saturating_mul(right).min(u64::MAX as u128) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_byte_values() {
        assert_eq!(format_bytes(0), "0 B");

        assert_eq!(format_bytes(1024), "1,0 KiB");

        assert_eq!(format_bytes(1024 * 1024 * 1024), "1,0 GiB");
    }

    #[test]
    fn classifies_disk_usage() {
        assert_eq!(usage_level(79.9), DiskUsageLevel::Normal);

        assert_eq!(usage_level(80.0), DiskUsageLevel::Warning);

        assert_eq!(usage_level(90.0), DiskUsageLevel::Critical);
    }
}
