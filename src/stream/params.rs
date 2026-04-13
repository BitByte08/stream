use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamParams {
    pub vo: String,
    pub hwdec: String,
    pub gpu_api: String,
    pub video_sync: String,
    pub interpolation: bool,
    pub ts_offset: f64,
    pub profile: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub cache: bool,
    pub cache_secs: u32,
    pub ao: String,
    pub audio_channels: u32,
    pub extra: Vec<String>,
}

impl StreamParams {
    pub fn new_4k60() -> Self {
        Self {
            vo: "gpu".to_string(),
            hwdec: "auto".to_string(),
            gpu_api: "opengl".to_string(),
            video_sync: "display-resample".to_string(),
            interpolation: true,
            ts_offset: 0.0,
            profile: "fast".to_string(),
            width: 3840,
            height: 2160,
            fps: 60,
            cache: true,
            cache_secs: 10,
            ao: "alsa".to_string(),
            audio_channels: 2,
            extra: vec![
                "--gpu-context=wayland".to_string(),
                "--opengl-es=yes".to_string(),
                "--vd-lavc-fast".to_string(),
                "--vd-lavc-skiploopfilter=nonref".to_string(),
                "--vd-lavc-skipframe=nonref".to_string(),
                "--vd-lavc-framedrop=nonref".to_string(),
                "--video-latency-hacks=yes".to_string(),
                "--demuxer-max-bytes=150MiB".to_string(),
                "--demuxer-max-back-bytes=75MiB".to_string(),
                "--demuxer-seekable-cache=no".to_string(),
            ],
        }
    }

    pub fn new_4k30() -> Self {
        let mut p = Self::new_4k60();
        p.fps = 30;
        p.interpolation = false;
        p.extra.retain(|e| !e.starts_with("--video-sync"));
        p
    }

    pub fn new_1080p60() -> Self {
        let mut p = Self::new_4k60();
        p.width = 1920;
        p.height = 1080;
        p.extra.retain(|e| !e.starts_with("--demuxer-max"));
        p.extra.push("--demuxer-max-bytes=50MiB".to_string());
        p.extra.push("--demuxer-max-back-bytes=25MiB".to_string());
        p
    }

    pub fn new_low_latency() -> Self {
        let mut p = Self::new_4k60();
        p.cache = false;
        p.interpolation = false;
        p.video_sync = "audio".to_string();
        p.extra = vec![
            "--profile=fast".to_string(),
            "--vd-lavc-fast".to_string(),
            "--vd-lavc-skiploopfilter=all".to_string(),
            "--vd-lavc-skipframe=nonkey".to_string(),
            "--vd-lavc-framedrop=all".to_string(),
            "--video-latency-hacks=yes".to_string(),
            "--no-cache".to_string(),
            "--untimed".to_string(),
            "--opengl-es=yes".to_string(),
        ];
        p
    }

    pub fn new_4k60_vulkan() -> Self {
        Self {
            vo: "gpu".to_string(),
            hwdec: "auto".to_string(),
            gpu_api: "vulkan".to_string(),
            video_sync: "display-resample".to_string(),
            interpolation: true,
            ts_offset: 0.0,
            profile: "fast".to_string(),
            width: 3840,
            height: 2160,
            fps: 60,
            cache: true,
            cache_secs: 10,
            ao: "alsa".to_string(),
            audio_channels: 2,
            extra: vec![
                "--gpu-context=waylandvk".to_string(),
                "--vd-lavc-fast".to_string(),
                "--vd-lavc-skiploopfilter=nonref".to_string(),
                "--vd-lavc-skipframe=nonref".to_string(),
                "--vd-lavc-framedrop=nonref".to_string(),
                "--video-latency-hacks=yes".to_string(),
                "--demuxer-max-bytes=150MiB".to_string(),
                "--demuxer-max-back-bytes=75MiB".to_string(),
                "--spirv-compiler=auto".to_string(),
            ],
        }
    }

    pub fn with_api(mut self, api: &str) -> Self {
        self.gpu_api = api.to_string();
        if api == "vulkan" {
            self.extra.retain(|e| !e.starts_with("--opengl"));
            self.extra.retain(|e| !e.starts_with("--gpu-context"));
            self.extra.push("--gpu-context=waylandvk".to_string());
            self.extra.push("--spirv-compiler=auto".to_string());
        }
        self
    }

    pub fn to_args(&self) -> Vec<String> {
        let mut args = vec![
            format!("--vo={}", self.vo),
            format!("--hwdec={}", self.hwdec),
            format!("--gpu-api={}", self.gpu_api),
            format!("--video-sync={}", self.video_sync),
            format!("--profile={}", self.profile),
            format!("--ao={}", self.ao),
            format!("--audio-channels={}", self.audio_channels),
        ];

        if self.interpolation {
            args.push("--interpolation".to_string());
        }

        if self.cache {
            args.push(format!("--cache={}", self.cache_secs));
        } else {
            args.push("--no-cache".to_string());
        }

        args.extend(self.extra.iter().cloned());
        args
    }
}
