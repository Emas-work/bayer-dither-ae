use pipl::*;

const PF_PLUG_IN_VERSION: u16 = 13;
const PF_PLUG_IN_SUBVERS: u16 = 28;

#[rustfmt::skip]
fn main() {
    // Catch panics so a bug shows an error in AE instead of crashing it.
    println!("cargo:rustc-cfg=catch_panics");

    pipl::plugin_build(vec![
        Property::Kind(PIPLType::AEEffect),
        Property::Name("Bayer Dither"),
        Property::Category("Plumcat"),

        #[cfg(target_os = "windows")]
        Property::CodeWin64X86("EffectMain"),
        #[cfg(target_os = "macos")]
        Property::CodeMacIntel64("EffectMain"),
        #[cfg(target_os = "macos")]
        Property::CodeMacARM64("EffectMain"),

        Property::AE_PiPL_Version { major: 2, minor: 0 },
        Property::AE_Effect_Spec_Version { major: PF_PLUG_IN_VERSION, minor: PF_PLUG_IN_SUBVERS },
        Property::AE_Effect_Version { version: 1, subversion: 0, bugversion: 0, stage: Stage::Release, build: 1 },
        Property::AE_Effect_Info_Flags(0),
        Property::AE_Effect_Global_OutFlags(
            OutFlags::DeepColorAware
        ),
        Property::AE_Effect_Global_OutFlags_2(
            OutFlags2::FloatColorAware |
            OutFlags2::SupportsSmartRender |
            OutFlags2::SupportsThreadedRendering |
            OutFlags2::SupportsGetFlattenedSequenceData
        ),
        Property::AE_Effect_Match_Name("PLUM Bayer Dither"),
        Property::AE_Reserved_Info(0),
        Property::AE_Effect_Support_URL("https://plumcat.studio"),
    ])
}
