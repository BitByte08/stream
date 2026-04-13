use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConfig {
    pub overlay: String,
    pub gpu_mem: u32,
    pub cma_size_mb: u32,
    pub hdmi_mode: HdmiMode,
    pub hdmi_cvt: HdmiCvt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HdmiMode {
    pub group: u32,
    pub mode: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HdmiCvt {
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub aspect: u32,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            overlay: "vc4-kms-v3d".to_string(),
            gpu_mem: 256,
            cma_size_mb: 512,
            hdmi_mode: HdmiMode { group: 2, mode: 16 },
            hdmi_cvt: HdmiCvt {
                width: 3840,
                height: 2160,
                framerate: 60,
                aspect: 16,
            },
        }
    }
}

impl GpuConfig {
    pub fn new_4k60() -> Self {
        Self::default()
    }

    pub fn config_lines(&self) -> Vec<String> {
        vec![
            format!("dtoverlay={}", self.overlay),
            format!("gpu_mem_1024={}", self.gpu_mem),
            format!("hdmi_group={}", self.hdmi_mode.group),
            format!("hdmi_mode={}", self.hdmi_mode.mode),
            format!(
                "hdmi_cvt {} {} {} {} 0 0 1",
                self.hdmi_cvt.width,
                self.hdmi_cvt.height,
                self.hdmi_cvt.framerate,
                self.hdmi_cvt.aspect,
            ),
            "hdmi_drive=2".to_string(),
            "disable_overscan=1".to_string(),
        ]
    }

    pub fn cmdline_params(&self) -> String {
        format!("cma={}M", self.cma_size_mb)
    }
}
