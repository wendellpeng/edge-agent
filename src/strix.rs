use serde::{Deserialize, Serialize};

/// RTSP URL 模式（对标 Strix popular patterns）
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RtspPattern {
    pub name: String,
    pub template: String,
    pub stream_type: String, // "main" | "sub"
}

/// 获取所有可用的 RTSP URL 模板
/// 优先级：
/// 1. 如果系统有 strix 命令，尝试调用它
/// 2. 如果存在 ~/.config/strix/cameras.db，读取 SQLite
/// 3. Fallback：内置 Strix 流行模式表（覆盖 90%+ 设备）
pub fn get_patterns() -> Vec<RtspPattern> {
    // TODO: 未来可扩展为从 Strix DB 动态加载
    // 目前使用内置的 Strix 流行模式表（精选最常用模式）
    built_in_popular_patterns()
}

/// 为指定参数生成候选 URL 列表
#[allow(dead_code)]
pub fn generate_urls(ip: &str, user: &str, pass: &str, channel: i32, patterns: &[RtspPattern]) -> Vec<(String, String)> {
    let mut urls = Vec::new();
    for p in patterns {
        let url = p.template
            .replace("{user}", user)
            .replace("{pass}", pass)
            .replace("{ip}", ip)
            .replace("{channel}", &channel.to_string());
        urls.push((p.name.clone(), url));
    }
    urls
}

/// 内置 Strix 流行模式表（基于 Strix 开源项目的 popular patterns）
/// 覆盖海康、大华、TP-Link、宇视、Axis、Sony、Panasonic、三星、通用等
fn built_in_popular_patterns() -> Vec<RtspPattern> {
    vec![
        // ===== 海康威视系列 =====
        RtspPattern {
            name: "hikvision_nvr_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/Streaming/Channels/{channel}01".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "hikvision_nvr_sub".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/Streaming/Channels/{channel}02".into(),
            stream_type: "sub".into(),
        },
        RtspPattern {
            name: "hikvision_ipc_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/streaming/channels/{channel}01".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "hikvision_old_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/h264/ch{channel}/main/av_stream".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "hikvision_old_sub".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/h264/ch{channel}/sub/av_stream".into(),
            stream_type: "sub".into(),
        },
        // ===== 大华系列 =====
        RtspPattern {
            name: "dahua_nvr_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/cam/realmonitor?channel={channel}&subtype=0".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "dahua_nvr_sub".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/cam/realmonitor?channel={channel}&subtype=1".into(),
            stream_type: "sub".into(),
        },
        RtspPattern {
            name: "dahua_ipc_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/live/ch{channel}".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "dahua_ipc_sub".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/live/ch{channel}_2".into(),
            stream_type: "sub".into(),
        },
        // ===== TP-Link / Tapo =====
        RtspPattern {
            name: "tplink_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/stream1".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "tplink_sub".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/stream2".into(),
            stream_type: "sub".into(),
        },
        RtspPattern {
            name: "tapo_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/stream{channel}".into(),
            stream_type: "main".into(),
        },
        // ===== 宇视 Uniview =====
        RtspPattern {
            name: "uniview_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/video{channel}".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "uniview_sub".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/video{channel}_2".into(),
            stream_type: "sub".into(),
        },
        // ===== Axis =====
        RtspPattern {
            name: "axis_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/axis-media/media.amp?videocodec=h264&streamprofile=Quality".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "axis_sub".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/axis-media/media.amp?videocodec=h264&streamprofile=Mobile".into(),
            stream_type: "sub".into(),
        },
        // ===== Sony =====
        RtspPattern {
            name: "sony_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/video{channel}".into(),
            stream_type: "main".into(),
        },
        // ===== Panasonic =====
        RtspPattern {
            name: "panasonic_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/MediaInput/h264/stream_{channel}".into(),
            stream_type: "main".into(),
        },
        // ===== Samsung / Hanwha =====
        RtspPattern {
            name: "samsung_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/profile{channel}/media.smp".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "hanwha_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/live/{channel}".into(),
            stream_type: "main".into(),
        },
        // ===== Bosch =====
        RtspPattern {
            name: "bosch_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/rtsp_tunnel".into(),
            stream_type: "main".into(),
        },
        // ===== Vivotek =====
        RtspPattern {
            name: "vivotek_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/live{channel}.sdp".into(),
            stream_type: "main".into(),
        },
        // ===== 雄迈 / XM =====
        RtspPattern {
            name: "xm_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/user={user}&password={pass}&channel={channel}&stream=0.sdp?".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "xm_sub".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/user={user}&password={pass}&channel={channel}&stream=1.sdp?".into(),
            stream_type: "sub".into(),
        },
        // ===== 安讯士通用 =====
        RtspPattern {
            name: "axis_generic".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/mpeg4/media.amp".into(),
            stream_type: "main".into(),
        },
        // ===== ONVIF 通用 Profile =====
        RtspPattern {
            name: "onvif_profile1".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/profile1".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "onvif_profile2".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/profile2".into(),
            stream_type: "sub".into(),
        },
        // ===== 通用 / 其他 =====
        RtspPattern {
            name: "generic_stream".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/stream{channel}".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "generic_live".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/live{channel}".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "generic_ch".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/ch{channel}".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "generic_cam".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/cam{channel}".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "generic_h264".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/h264".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "generic_mpeg4".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/mpeg4".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "generic_media".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/media/video{channel}".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "generic_rtsp".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "generic_play".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/play{channel}".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "generic_chn".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/chn{channel}".into(),
            stream_type: "main".into(),
        },
        // ===== 中维 / Jovision =====
        RtspPattern {
            name: "jovision_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/0/{channel}".into(),
            stream_type: "main".into(),
        },
        // ===== 天地伟业 / Tiandy =====
        RtspPattern {
            name: "tiandy_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/1/h264major".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "tiandy_sub".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/1/h264minor".into(),
            stream_type: "sub".into(),
        },
        // ===== 科达 / Kedacom =====
        RtspPattern {
            name: "kedacom_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/defaultPrimary?streamType=u".into(),
            stream_type: "main".into(),
        },
        // ===== 蓝色星际 / BlueStar =====
        RtspPattern {
            name: "bluestar_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/user={user}&password={pass}&channel={channel}&stream=0.sdp".into(),
            stream_type: "main".into(),
        },
        // ===== 英飞拓 / Infinova =====
        RtspPattern {
            name: "infinova_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/1".into(),
            stream_type: "main".into(),
        },
        // ===== 亚安 / YaAn =====
        RtspPattern {
            name: "yaan_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/live{channel}.sdp".into(),
            stream_type: "main".into(),
        },
        // ===== 迅驰 / XunChi =====
        RtspPattern {
            name: "xunchi_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/0".into(),
            stream_type: "main".into(),
        },
        // ===== 汉邦高科 / Hanbang =====
        RtspPattern {
            name: "hanbang_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/0/{channel}/live".into(),
            stream_type: "main".into(),
        },
        // ===== 同为 / Tvt =====
        RtspPattern {
            name: "tvt_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/Streaming/Channels/{channel}01".into(),
            stream_type: "main".into(),
        },
        // ===== 龙视安 / Loosafe =====
        RtspPattern {
            name: "loosafe_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/11".into(),
            stream_type: "main".into(),
        },
        // ===== 睿视 / Reolink =====
        RtspPattern {
            name: "reolink_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/h264Preview_01_main".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "reolink_sub".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/h264Preview_01_sub".into(),
            stream_type: "sub".into(),
        },
        // ===== Amcrest / Foscam =====
        RtspPattern {
            name: "amcrest_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/cam/realmonitor?channel=1&subtype=0".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "foscam_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/videoMain".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "foscam_sub".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/videoSub".into(),
            stream_type: "sub".into(),
        },
        // ===== D-Link =====
        RtspPattern {
            name: "dlink_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/play{channel}.sdp".into(),
            stream_type: "main".into(),
        },
        // ===== Edimax =====
        RtspPattern {
            name: "edimax_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/ipcam_h264.sdp".into(),
            stream_type: "main".into(),
        },
        // ===== Genetec =====
        RtspPattern {
            name: "genetec_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/live.sdp".into(),
            stream_type: "main".into(),
        },
        // ===== Hikvision SDK RTSP =====
        RtspPattern {
            name: "hikvision_sdk".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/ISAPI/Streaming/channels/{channel}01".into(),
            stream_type: "main".into(),
        },
        // ===== ONVIF fallback =====
        RtspPattern {
            name: "onvif_fallback".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/".into(),
            stream_type: "main".into(),
        },
        // ===== 萤石 / EZVIZ（海康子品牌）=====
        RtspPattern {
            name: "ezviz_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/h264/ch{channel}/main/av_stream".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "ezviz_sub".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/h264/ch{channel}/sub/av_stream".into(),
            stream_type: "sub".into(),
        },
        // ===== 小米 / 米家 / 小白 =====
        RtspPattern {
            name: "xiaomi_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/live/ch{channel}".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "xiaomi_sub".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/live/ch{channel}_2".into(),
            stream_type: "sub".into(),
        },
        // ===== 华为 / HiLook =====
        RtspPattern {
            name: "huawei_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/LiveMedia/ch{channel}/Media1".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "huawei_sub".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/LiveMedia/ch{channel}/Media2".into(),
            stream_type: "sub".into(),
        },
        // ===== 乐橙 / Imou（大华子品牌）=====
        RtspPattern {
            name: "imou_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/cam/realmonitor?channel={channel}&subtype=0".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "imou_sub".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/cam/realmonitor?channel={channel}&subtype=1".into(),
            stream_type: "sub".into(),
        },
        // ===== 360 =====
        RtspPattern {
            name: "360_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/live/ch{channel}".into(),
            stream_type: "main".into(),
        },
        // ===== 乔安 / Jooan =====
        RtspPattern {
            name: "jooan_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/user={user}&password={pass}&channel={channel}&stream=0.sdp?".into(),
            stream_type: "main".into(),
        },
        // ===== 沃仕达 / Wanscam =====
        RtspPattern {
            name: "wanscam_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/live{channel}.sdp".into(),
            stream_type: "main".into(),
        },
        // ===== 帝防 / Defeway =====
        RtspPattern {
            name: "defeway_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/user={user}_password={pass}_channel={channel}_stream=0.sdp".into(),
            stream_type: "main".into(),
        },
        // ===== 更多海康变体（不同固件版本路径差异）=====
        RtspPattern {
            name: "hikvision_alt1".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/Streaming/Channels/{channel}".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "hikvision_alt2".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/streaming/channels/{channel}01".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "hikvision_alt3".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/PSIA/streaming/channels/{channel}01".into(),
            stream_type: "main".into(),
        },
        // ===== 更多大华变体 =====
        RtspPattern {
            name: "dahua_alt1".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/live/ch{channel}".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "dahua_alt2".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/cam/realmonitor?channel={channel}&subtype=0&unicast=true&proto=Onvif".into(),
            stream_type: "main".into(),
        },
        // ===== 更多通用变体（无通道号 / 固定路径）=====
        RtspPattern {
            name: "generic_stream1".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/stream1".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "generic_stream2".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/stream2".into(),
            stream_type: "sub".into(),
        },
        RtspPattern {
            name: "generic_live1".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/live1".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "generic_live2".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/live2".into(),
            stream_type: "sub".into(),
        },
        RtspPattern {
            name: "generic_video1".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/video1".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "generic_video2".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/video2".into(),
            stream_type: "sub".into(),
        },
        RtspPattern {
            name: "generic_h264_1".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/h264_1".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "generic_h264_2".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/h264_2".into(),
            stream_type: "sub".into(),
        },
        RtspPattern {
            name: "generic_0".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/0".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "generic_1".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/1".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "generic_11".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/11".into(),
            stream_type: "main".into(),
        },
        // ===== ONVIF 标准 Profile 变体 =====
        RtspPattern {
            name: "onvif_profile_main".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/profile1".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "onvif_profile_sub".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/profile2".into(),
            stream_type: "sub".into(),
        },
        RtspPattern {
            name: "onvif_profile3".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/profile3".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "onvif_profile4".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/profile4".into(),
            stream_type: "sub".into(),
        },
        // ===== 海康 ISAPI 变体 =====
        RtspPattern {
            name: "hikvision_isapi".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/ISAPI/Streaming/channels/{channel}01".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "hikvision_isapi2".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/ISAPI/ContentMgmt/StreamingProxy/channels/{channel}01".into(),
            stream_type: "main".into(),
        },
        // ===== 更多品牌变体 =====
        RtspPattern {
            name: "tvt_alt".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/streaming/channels/{channel}01".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "samsung_alt".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/profile{channel}/media.smp".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "axis_alt".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/axis-media/media.amp".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "vivotek_alt".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/live.sdp".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "bosch_alt".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/video".into(),
            stream_type: "main".into(),
        },
        RtspPattern {
            name: "genetec_alt".into(),
            template: "rtsp://{user}:{pass}@{ip}:554/".into(),
            stream_type: "main".into(),
        },
    ]
}

/// 根据成功匹配的模式名称猜测品牌
pub fn guess_brand_from_pattern(pattern_name: &str) -> String {
    if pattern_name.starts_with("hikvision") || pattern_name.starts_with("hikvision_sdk") {
        "hikvision".into()
    } else if pattern_name.starts_with("dahua") {
        "dahua".into()
    } else if pattern_name.starts_with("tplink") || pattern_name.starts_with("tapo") {
        "tp-link".into()
    } else if pattern_name.starts_with("uniview") {
        "uniview".into()
    } else if pattern_name.starts_with("axis") {
        "axis".into()
    } else if pattern_name.starts_with("sony") {
        "sony".into()
    } else if pattern_name.starts_with("panasonic") {
        "panasonic".into()
    } else if pattern_name.starts_with("samsung") || pattern_name.starts_with("hanwha") {
        "hanwha".into()
    } else if pattern_name.starts_with("bosch") {
        "bosch".into()
    } else if pattern_name.starts_with("vivotek") {
        "vivotek".into()
    } else if pattern_name.starts_with("xm") {
        "xm".into()
    } else if pattern_name.starts_with("jovision") {
        "jovision".into()
    } else if pattern_name.starts_with("tiandy") {
        "tiandy".into()
    } else if pattern_name.starts_with("kedacom") {
        "kedacom".into()
    } else if pattern_name.starts_with("bluestar") {
        "bluestar".into()
    } else if pattern_name.starts_with("infinova") {
        "infinova".into()
    } else if pattern_name.starts_with("yaan") {
        "yaan".into()
    } else if pattern_name.starts_with("xunchi") {
        "xunchi".into()
    } else if pattern_name.starts_with("hanbang") {
        "hanbang".into()
    } else if pattern_name.starts_with("tvt") {
        "tvt".into()
    } else if pattern_name.starts_with("loosafe") {
        "loosafe".into()
    } else if pattern_name.starts_with("reolink") {
        "reolink".into()
    } else if pattern_name.starts_with("amcrest") {
        "amcrest".into()
    } else if pattern_name.starts_with("foscam") {
        "foscam".into()
    } else if pattern_name.starts_with("dlink") {
        "dlink".into()
    } else if pattern_name.starts_with("ezviz") {
        "ezviz".into()
    } else if pattern_name.starts_with("xiaomi") {
        "xiaomi".into()
    } else if pattern_name.starts_with("huawei") {
        "huawei".into()
    } else if pattern_name.starts_with("imou") {
        "imou".into()
    } else if pattern_name.starts_with("360") {
        "360".into()
    } else if pattern_name.starts_with("jooan") {
        "jooan".into()
    } else if pattern_name.starts_with("wanscam") {
        "wanscam".into()
    } else if pattern_name.starts_with("defeway") {
        "defeway".into()
    } else if pattern_name.starts_with("edimax") {
        "edimax".into()
    } else if pattern_name.starts_with("genetec") {
        "genetec".into()
    } else {
        "unknown".into()
    }
}
