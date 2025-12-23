// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Fix for WSL2 WebKit compatibility issues
    #[cfg(target_os = "linux")]
    {
        // Disable WebKit DMA-BUF renderer which can cause issues in WSL2
        if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }

    devflow_lib::run()
}
