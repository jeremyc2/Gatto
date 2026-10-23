//! Native dictionary schemas, kept separate from safe policy and scheduling.
use super::*;
pub(super) fn video_decode_settings() -> Retained<NSDictionary<NSString, AnyObject>> {
    // SAFETY: Keys and value classes follow Apple's documented codec schema;
    // these exported constants exist on the minimum supported macOS version.
    unsafe {
        let pixel_key = NSString::from_str(&kCVPixelBufferPixelFormatTypeKey.to_string());
        dictionary(&[(
            &pixel_key,
            &*NSNumber::new_u32(kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange),
        )])
    }
}
pub(super) fn audio_decode_settings() -> Retained<NSDictionary<NSString, AnyObject>> {
    // SAFETY: Keys and value classes follow Apple's documented codec schema;
    // these exported constants exist on the minimum supported macOS version.
    unsafe {
        dictionary(&[
            (
                AVFormatIDKey.unwrap(),
                &*NSNumber::new_u32(u32::from_be_bytes(*b"lpcm")),
            ),
            (AVSampleRateKey.unwrap(), &*NSNumber::new_u32(48_000)),
            (AVNumberOfChannelsKey.unwrap(), &*NSNumber::new_u32(1)),
            (AVLinearPCMBitDepthKey.unwrap(), &*NSNumber::new_u32(16)),
            (AVLinearPCMIsFloatKey.unwrap(), &*NSNumber::new_bool(false)),
            (
                AVLinearPCMIsBigEndianKey.unwrap(),
                &*NSNumber::new_bool(false),
            ),
            (
                AVLinearPCMIsNonInterleaved.unwrap(),
                &*NSNumber::new_bool(false),
            ),
        ])
    }
}
pub(super) fn video_encode_settings(
    bitrate: u32,
    width: usize,
    height: usize,
) -> Retained<NSDictionary<NSString, AnyObject>> {
    // SAFETY: Keys and value classes follow Apple's documented codec schema;
    // these exported constants exist on the minimum supported macOS version.
    unsafe {
        // AVAssetWriter passes these documented settings to native
        // VideoToolbox: ABR targets bits/sec, FPS is a source hint, and
        // GOP caps both frame count and elapsed time. ABR is not a cap.
        let compression = dictionary(&[
            (
                AVVideoAverageBitRateKey.unwrap(),
                &*NSNumber::new_u32(bitrate),
            ),
            (
                AVVideoExpectedSourceFrameRateKey.unwrap(),
                &*NSNumber::new_u32(30),
            ),
            (
                AVVideoMaxKeyFrameIntervalKey.unwrap(),
                &*NSNumber::new_u32(300),
            ),
            (
                AVVideoMaxKeyFrameIntervalDurationKey.unwrap(),
                &*NSNumber::new_u32(10),
            ),
            (
                AVVideoProfileLevelKey.unwrap(),
                AVVideoProfileLevelH264HighAutoLevel.unwrap(),
            ),
            (
                AVVideoAllowFrameReorderingKey.unwrap(),
                &*NSNumber::new_bool(false),
            ),
        ]);
        // The encoder specification is TOP LEVEL, not a compression
        // property. Requiring hardware forbids a silent CPU fallback.
        let hardware_key = NSString::from_str(&objc2_video_toolbox::kVTVideoEncoderSpecification_RequireHardwareAcceleratedVideoEncoder.to_string());
        let hardware = dictionary(&[(&hardware_key, &*NSNumber::new_bool(true))]);
        dictionary(&[
            (AVVideoCodecKey.unwrap(), AVVideoCodecTypeH264.unwrap()),
            (AVVideoWidthKey.unwrap(), &*NSNumber::new_usize(width)),
            (AVVideoHeightKey.unwrap(), &*NSNumber::new_usize(height)),
            (AVVideoCompressionPropertiesKey.unwrap(), &*compression),
            (AVVideoEncoderSpecificationKey.unwrap(), &*hardware),
        ])
    }
}
pub(super) fn audio_encode_settings() -> Retained<NSDictionary<NSString, AnyObject>> {
    // SAFETY: Keys and value classes follow Apple's documented codec schema;
    // these exported constants exist on the minimum supported macOS version.
    unsafe {
        dictionary(&[
            (
                AVFormatIDKey.unwrap(),
                &*NSNumber::new_u32(u32::from_be_bytes(*b"aac ")),
            ),
            (AVEncoderBitRateKey.unwrap(), &*NSNumber::new_u32(64_000)),
            (AVSampleRateKey.unwrap(), &*NSNumber::new_u32(48_000)),
            (AVNumberOfChannelsKey.unwrap(), &*NSNumber::new_u32(1)),
        ])
    }
}
