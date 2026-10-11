//! 注册：COM InProcServer（HKLM）+ TSF 输入法档案 + 16 项类别组。

use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::UI::TextServices::{
    CLSID_TF_CategoryMgr, CLSID_TF_InputProcessorProfiles, ITfCategoryMgr,
    ITfInputProcessorProfiles, GUID_TFCAT_CATEGORY_OF_TIP, GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
    GUID_TFCAT_DISPLAYATTRIBUTEPROPERTY, GUID_TFCAT_PROPSTYLE_STATIC, GUID_TFCAT_PROP_AUDIODATA,
    GUID_TFCAT_PROP_INKDATA, GUID_TFCAT_TIPCAP_COMLESS, GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
    GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT, GUID_TFCAT_TIPCAP_SECUREMODE,
    GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT, GUID_TFCAT_TIPCAP_UIELEMENTENABLED, GUID_TFCAT_TIP_KEYBOARD,
};
use windows::core::GUID;
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
use winreg::HKEY;

use crate::global::{self, IME_ID, IME_NAME, IME_NAME_ASCII, LANGID_ZH_CN, LANG_PROFILE_ID, to_widez, zh_cn_us_hkl};

/// 输入法支持的特性组。Speech/Handwriting 除外，其余全部注册，
/// 否则 Word/Office 等应用中输入法不工作。
const SUPPORTED_CATEGORIES: [GUID; 15] = [
    GUID_TFCAT_CATEGORY_OF_TIP,
    GUID_TFCAT_TIP_KEYBOARD,
    GUID_TFCAT_TIPCAP_SECUREMODE,
    GUID_TFCAT_TIPCAP_UIELEMENTENABLED,
    GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT,
    GUID_TFCAT_TIPCAP_COMLESS,
    GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
    GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,
    GUID_TFCAT_PROP_AUDIODATA,
    GUID_TFCAT_PROP_INKDATA,
    GUID_TFCAT_PROPSTYLE_STATIC,
    GUID::from_u128(0x85F9794B_4D19_40D8_8864_4E747371A66D), // PROPSTYLE_STATICCOMPSCT
    GUID::from_u128(0x24AF3031_852D_40A2_BC09_8992898CE722), // PROPSTYLE_CUSTOM
    GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
    GUID_TFCAT_DISPLAYATTRIBUTEPROPERTY,
];

/// 第一步：注册 COM InProcServer。
/// 优先 HKLM（需管理员）；失败则尝试 HKCU（per-user，免提权）。
fn register_server() -> windows::core::Result<()> {
    let dll_path = global::dll_path().ok_or(windows::core::Error::from(
        windows::Win32::Foundation::E_FAIL,
    ))?;
    for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        if try_register_server(hive, &dll_path).is_ok() {
            log::info!("COM InProcServer 注册到 {:?}", hive);
            return Ok(());
        }
    }
    Err(windows::core::Error::from(
        windows::Win32::Foundation::E_FAIL,
    ))
}

fn try_register_server(hive: HKEY, dll_path: &str) -> std::io::Result<()> {
    let root = RegKey::predef(hive);
    let path = format!("SOFTWARE\\Classes\\CLSID\\{{{}}}", IME_ID.to_rfc4122_str());
    let (clsid, _) = root.create_subkey(path)?;
    clsid.set_value("", &IME_NAME_ASCII)?;
    let (inproc, _) = clsid.create_subkey("InProcServer32")?;
    inproc.set_value("", &dll_path)?;
    inproc.set_value("ThreadingModel", &"Apartment")?;
    Ok(())
}

fn unregister_server() -> windows::core::Result<()> {
    for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        let root = RegKey::predef(hive);
        let path = format!("SOFTWARE\\Classes\\CLSID\\{{{}}}", IME_ID.to_rfc4122_str());
        let _ = root.delete_subkey_all(path);
    }
    Ok(())
}

/// 第二步：注册 TSF 输入法 + 语言档案 + 类别组。
fn register_ime() -> windows::core::Result<()> {
    unsafe {
        let profiles: ITfInputProcessorProfiles =
            CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?;
        let categories: ITfCategoryMgr =
            CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)?;

        profiles.Register(&IME_ID)?;
        log::info!("已注册输入处理器");

        let name = to_widez(IME_NAME);
        let icon = to_widez(global::dll_path().as_deref().unwrap_or(""));
        profiles.AddLanguageProfile(
            &IME_ID,
            LANGID_ZH_CN,
            &LANG_PROFILE_ID,
            &name,
            &icon,
            0,
        )?;
        profiles.SubstituteKeyboardLayout(
            &IME_ID,
            LANGID_ZH_CN,
            &LANG_PROFILE_ID,
            zh_cn_us_hkl(),
        )?;
        log::info!("已添加语言档案");

        for cat in SUPPORTED_CATEGORIES {
            categories.RegisterCategory(&IME_ID, &cat, &IME_ID)?;
        }
        log::info!("已注册 {} 项类别", SUPPORTED_CATEGORIES.len());
    }
    Ok(())
}

fn unregister_ime() -> windows::core::Result<()> {
    unsafe {
        let profiles: ITfInputProcessorProfiles =
            CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?;
        let categories: ITfCategoryMgr =
            CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)?;
        for cat in SUPPORTED_CATEGORIES {
            let _ = categories.UnregisterCategory(&IME_ID, &cat, &IME_ID);
        }
        let _ = profiles.RemoveLanguageProfile(&IME_ID, LANGID_ZH_CN, &LANG_PROFILE_ID);
        profiles.Unregister(&IME_ID)?;
        log::info!("已反注册输入法");
    }
    Ok(())
}

pub fn register_all() -> windows::core::Result<()> {
    register_server()?;
    register_ime()?;
    log::info!("注册完成，按 Win+Space 选择“{}”", IME_NAME);
    Ok(())
}

pub fn unregister_all() -> windows::core::Result<()> {
    unregister_ime()?;
    unregister_server()?;
    Ok(())
}

/// GUID 的 RFC4122 大写格式（winreg 需要）。
trait Rfc4122 {
    fn to_rfc4122_str(&self) -> String;
}

impl Rfc4122 for GUID {
    fn to_rfc4122_str(&self) -> String {
        let hex = format!("{:032X}", self.to_u128());
        format!(
            "{}-{}-{}-{}-{}",
            &hex[0..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..32]
        )
    }
}
