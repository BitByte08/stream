#[cfg(test)]
mod tests {
    use stream_cli::dep::{optional_packages, required_packages};
    use stream_cli::gpu::config::GpuConfig;
    use stream_cli::stream::params::StreamParams;
    use stream_cli::transcode::TranscodeConfig;

    #[test]
    fn test_gpu_config_default() {
        let config = GpuConfig::default();
        assert_eq!(config.overlay, "vc4-kms-v3d");
        assert_eq!(config.gpu_mem, 256);
        assert_eq!(config.cma_size_mb, 512);
        assert_eq!(config.hdmi_cvt.width, 3840);
        assert_eq!(config.hdmi_cvt.height, 2160);
        assert_eq!(config.hdmi_cvt.framerate, 60);
    }

    #[test]
    fn test_gpu_config_lines() {
        let config = GpuConfig::new_4k60();
        let lines = config.config_lines();
        assert!(lines.contains(&"dtoverlay=vc4-kms-v3d".to_string()));
        assert!(lines.contains(&"gpu_mem_1024=256".to_string()));
        assert!(lines.contains(&"hdmi_group=2".to_string()));
        assert!(lines.contains(&"hdmi_mode=16".to_string()));
    }

    #[test]
    fn test_stream_params_4k60() {
        let params = StreamParams::new_4k60();
        assert_eq!(params.vo, "gpu");
        assert_eq!(params.hwdec, "auto");
        assert_eq!(params.gpu_api, "opengl");
        assert_eq!(params.width, 3840);
        assert_eq!(params.height, 2160);
        assert_eq!(params.fps, 60);
        assert!(params.cache);
        assert!(params.interpolation);
    }

    #[test]
    fn test_stream_params_4k30() {
        let params = StreamParams::new_4k30();
        assert_eq!(params.fps, 30);
        assert!(!params.interpolation);
    }

    #[test]
    fn test_stream_params_1080p60() {
        let params = StreamParams::new_1080p60();
        assert_eq!(params.width, 1920);
        assert_eq!(params.height, 1080);
        assert_eq!(params.fps, 60);
    }

    #[test]
    fn test_stream_params_low_latency() {
        let params = StreamParams::new_low_latency();
        assert!(!params.cache);
        assert!(!params.interpolation);
        assert_eq!(params.video_sync, "audio");
    }

    #[test]
    fn test_stream_params_vulkan() {
        let params = StreamParams::new_4k60_vulkan();
        assert_eq!(params.gpu_api, "vulkan");
        let has_spirv: bool = params.extra.iter().any(|e: &String| e.contains("spirv"));
        assert!(has_spirv);
    }

    #[test]
    fn test_stream_params_with_api() {
        let params = StreamParams::new_4k60().with_api("vulkan");
        assert_eq!(params.gpu_api, "vulkan");
        let has_spirv: bool = params.extra.iter().any(|e: &String| e.contains("spirv"));
        assert!(has_spirv);
    }

    #[test]
    fn test_stream_params_to_args() {
        let params = StreamParams::new_4k60();
        let args = params.to_args();
        assert!(args.contains(&"--vo=gpu".to_string()));
        assert!(args.contains(&"--hwdec=auto".to_string()));
        assert!(args.contains(&"--gpu-api=opengl".to_string()));
        assert!(args.contains(&"--interpolation".to_string()));
    }

    #[test]
    fn test_required_packages() {
        let pkgs = required_packages();
        assert!(!pkgs.is_empty());
        assert!(pkgs.contains(&"mpv"));
        assert!(pkgs.contains(&"ffmpeg"));
    }

    #[test]
    fn test_optional_packages() {
        let pkgs = optional_packages();
        assert!(pkgs.contains(&"v4l-utils"));
    }

    #[test]
    fn test_stream_params_serialization() {
        let params = StreamParams::new_4k60();
        let json = serde_json::to_string(&params).unwrap();
        let decoded: StreamParams = serde_json::from_str(&json).unwrap();
        assert_eq!(params.vo, decoded.vo);
        assert_eq!(params.width, decoded.width);
    }

    #[test]
    fn test_gpu_config_serialization() {
        let config = GpuConfig::new_4k60();
        let json = serde_json::to_string(&config).unwrap();
        let decoded: GpuConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.gpu_mem, decoded.gpu_mem);
        assert_eq!(config.hdmi_cvt.framerate, decoded.hdmi_cvt.framerate);
    }

    #[test]
    fn test_cmdline_params() {
        let config = GpuConfig::new_4k60();
        assert_eq!(config.cmdline_params(), "cma=512M");
    }

    #[test]
    fn test_stream_params_low_latency_no_cache() {
        let params = StreamParams::new_low_latency();
        let args = params.to_args();
        assert!(args.contains(&"--no-cache".to_string()));
        assert!(args.contains(&"--untimed".to_string()));
    }

    #[test]
    fn test_stream_params_1080p_demuxer() {
        let params = StreamParams::new_1080p60();
        let has_50: bool = params
            .extra
            .iter()
            .any(|e: &String| e == "--demuxer-max-bytes=50MiB");
        assert!(has_50);
    }

    #[test]
    fn test_vulkan_context() {
        let params = StreamParams::new_4k60_vulkan();
        let has_vk: bool = params
            .extra
            .iter()
            .any(|e: &String| e.contains("waylandvk"));
        assert!(has_vk);
    }

    #[test]
    fn test_transcode_config_default() {
        let config = TranscodeConfig::default();
        assert_eq!(config.codec, "hevc_v4l2m2m");
        assert_eq!(config.crf, 23);
        assert!(config.hw_accel);
        assert!(config.copy_audio);
        assert_eq!(config.container, "mp4");
    }

    #[test]
    fn test_transcode_config_new() {
        let config = TranscodeConfig::new_h264_to_h265("/tmp/test.mp4", None);
        assert!(config.input.to_str().unwrap().contains("test.mp4"));
        assert!(config.output.to_str().unwrap().contains("test_h265.mp4"));
        assert!(config.hw_accel);
    }

    #[test]
    fn test_transcode_config_with_output() {
        let config = TranscodeConfig::new_h264_to_h265("/tmp/test.mp4", Some("/tmp/output.mkv"));
        assert_eq!(config.output.to_str(), Some("/tmp/output.mkv"));
    }

    #[test]
    fn test_transcode_config_with_crf() {
        let config = TranscodeConfig::default().with_crf(18);
        assert_eq!(config.crf, 18);

        let config_max = TranscodeConfig::default().with_crf(100);
        assert_eq!(config_max.crf, 51);
    }

    #[test]
    fn test_transcode_config_with_resolution() {
        let config = TranscodeConfig::default().with_resolution(1920, 1080);
        assert_eq!(config.width, Some(1920));
        assert_eq!(config.height, Some(1080));
    }

    #[test]
    fn test_transcode_config_software_fallback() {
        let config = TranscodeConfig::default().with_software_fallback();
        assert!(!config.hw_accel);
        assert_eq!(config.codec, "libx265");
    }

    #[test]
    fn test_transcode_ffmpeg_args_hw() {
        let config = TranscodeConfig::new_h264_to_h265("/input.mp4", Some("/output.mp4"));
        let args = config.to_ffmpeg_args();
        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"/input.mp4".to_string()));
        assert!(args.contains(&"-c:v".to_string()));
        assert!(args.contains(&"hevc_v4l2m2m".to_string()));
        assert!(args.contains(&"-c:a".to_string()));
        assert!(args.contains(&"copy".to_string()));
        assert!(args.contains(&"-movflags".to_string()));
        assert!(args.contains(&"+faststart".to_string()));
    }

    #[test]
    fn test_transcode_ffmpeg_args_sw() {
        let config = TranscodeConfig::new_h264_to_h265("/input.mp4", None)
            .with_software_fallback()
            .with_crf(20);
        let args = config.to_ffmpeg_args();
        assert!(args.contains(&"libx265".to_string()));
        assert!(args.contains(&"-preset".to_string()));
        assert!(args.contains(&"-crf".to_string()));
        assert!(args.contains(&"20".to_string()));
    }

    #[test]
    fn test_transcode_ffmpeg_args_resolution() {
        let config = TranscodeConfig::default().with_resolution(3840, 2160);
        let args = config.to_ffmpeg_args();
        let has_scale: bool = args.iter().any(|a: &String| a.starts_with("scale="));
        assert!(has_scale);
    }

    #[test]
    fn test_transcode_serialization() {
        let config = TranscodeConfig::new_h264_to_h265("/test.mp4", None);
        let json = serde_json::to_string(&config).unwrap();
        let decoded: TranscodeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.hw_accel, decoded.hw_accel);
        assert_eq!(config.crf, decoded.crf);
    }
}
