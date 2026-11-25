//! Global host state (set once at host start; defaults in unit tests).

use std::sync::OnceLock;

use crate::config::{Boot, Config};

static BOOT: OnceLock<Boot> = OnceLock::new();
static DEFAULT_CONFIG: OnceLock<Config> = OnceLock::new();

pub fn set_boot(b: Boot) {
    let _ = BOOT.set(b);
}

pub fn boot() -> Option<&'static Boot> {
    BOOT.get()
}

pub fn config() -> &'static Config {
    match BOOT.get() {
        Some(b) => &b.config,
        None => DEFAULT_CONFIG.get_or_init(Config::default),
    }
}

/// The title memory budget on this host (MiB): the configured one, the standard figure lowered
/// to the GPU's video memory (`Config::title_memory_for_host`). Worked out once: the host does
/// so when it starts, before the title runs. What a title was told must not change afterwards,
/// and no title thread has to ask DXGI in the middle of a call.
pub fn title_memory_budget_mb() -> u64 {
    static BUDGET: OnceLock<u64> = OnceLock::new();
    *BUDGET.get_or_init(|| {
        let c = config();
        let standard = c.title_memory_budget_mb();
        // only a resolved configuration with the standard budget asks the GPU (unit tests and
        // a figure from xcl.toml do not)
        if !c.title_memory_standard {
            return standard;
        }
        let Some((dedicated, shared, name)) = crate::gpu::adapter::video_memory_mb(&c.d3d12_adapter) else {
            return standard;
        };
        let vram = crate::config::host_video_memory_mb(dedicated, shared);
        // an integrated adapter is named with what its figure is made of
        let name = if vram != dedicated { format!("{name} (integrated: dedicated and shared memory)") } else { name };
        let fit = c.title_memory_fits_host();
        let budget = c.title_memory_for_host(vram, fit);
        if budget < standard {
            crate::xlog_info!(
                "title memory: {budget} MiB instead of the console's {standard} MiB – {name} has {vram} MiB of video memory (title_memory_mb in xcl.toml sets a figure, title_memory_fit = false keeps the console's)"
            );
        } else if standard > vram {
            crate::xlog_warn!(
                "title memory: the console's {standard} MiB is more than the {vram} MiB of video memory of {name} – the title may plan for more textures than fit (title_memory_fit = true lowers it, title_memory_mb sets a figure)"
            );
        }
        budget
    })
}

/// Folder for title data (saves, PLS, temp…) – a temp folder when no boot record.
pub fn data_dir() -> std::path::PathBuf {
    match BOOT.get() {
        Some(b) => b.data_dir.clone(),
        None => std::env::temp_dir().join("xcl_unit_data"),
    }
}
