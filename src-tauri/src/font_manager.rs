use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zip::ZipArchive;

use crate::{
    domain::{FontPackState, FontPackStatus},
    error::{AppError, AppResult},
};

pub(crate) const TERMES_PACK_ID: &str = "tex-gyre-termes";
pub(crate) const CHARIS_PACK_ID: &str = "charis-sil";
pub(crate) const NOTO_SERIF_PACK_ID: &str = "noto-serif";
pub(crate) const NOTO_CJK_TC_PACK_ID: &str = "noto-serif-cjk-tc";
pub(crate) const CHIRON_SUNG_HK_PACK_ID: &str = "chiron-sung-hk";
pub(crate) const CHIRON_HEI_HK_PACK_ID: &str = "chiron-hei-hk";

const NOTO_COMMIT: &str = "341cc991ffa33bb58fd0cb08728c6c6ac6c3b19a";

#[derive(Clone, Copy)]
struct PackFile {
    output: &'static str,
    archive_path: Option<&'static str>,
    url: Option<&'static str>,
    sha256: &'static str,
}

#[derive(Clone, Copy)]
enum PackSource {
    Archive {
        url: &'static str,
        sha256: &'static str,
    },
    Files,
}

#[derive(Clone, Copy)]
struct Pack {
    id: &'static str,
    version: &'static str,
    mandatory: bool,
    source: PackSource,
    files: &'static [PackFile],
    family: LatexFontFamily,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LatexFontFamily {
    pub(crate) regular: &'static str,
    pub(crate) bold: Option<&'static str>,
    pub(crate) italic: Option<&'static str>,
    pub(crate) bold_italic: Option<&'static str>,
}

trait Downloader: Send + Sync {
    fn download(&self, url: &str) -> AppResult<Vec<u8>>;
}

struct HttpDownloader;

impl Downloader for HttpDownloader {
    fn download(&self, url: &str) -> AppResult<Vec<u8>> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(120))
            .user_agent(concat!("bkuw/", env!("CARGO_PKG_VERSION"), " font-manager"))
            .build()
            .map_err(|error| font_error("font_download", "create HTTP client", error))?;
        let response = client
            .get(url)
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|error| font_error("font_download", "download font pack", error))?;
        response
            .bytes()
            .map(|bytes| bytes.to_vec())
            .map_err(|error| font_error("font_download", "read font pack response", error))
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstalledManifest {
    id: String,
    version: String,
    files: BTreeMap<String, String>,
}

#[derive(Clone)]
pub(crate) struct FontManager {
    root: PathBuf,
    downloader: Arc<dyn Downloader>,
    verification: VerificationMode,
}

#[derive(Clone, Copy)]
enum VerificationMode {
    Catalog,
    #[cfg(test)]
    TestManifest,
}

impl FontManager {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self {
            root,
            downloader: Arc::new(HttpDownloader),
            verification: VerificationMode::Catalog,
        }
    }

    #[cfg(test)]
    fn with_downloader(root: PathBuf, downloader: Arc<dyn Downloader>) -> Self {
        Self {
            root,
            downloader,
            verification: VerificationMode::Catalog,
        }
    }

    pub(crate) fn statuses(&self) -> Vec<FontPackStatus> {
        catalog().iter().map(|pack| self.status(pack)).collect()
    }

    pub(crate) fn status_for(&self, pack_id: &str) -> AppResult<FontPackStatus> {
        let pack = find_pack(pack_id)?;
        Ok(self.status(pack))
    }

    pub(crate) fn install(&self, pack_id: &str) -> AppResult<FontPackStatus> {
        let pack = find_pack(pack_id)?;
        fs::create_dir_all(&self.root)
            .map_err(|error| font_error("font_filesystem", "create font cache", error))?;
        let staging = tempfile::Builder::new()
            .prefix(&format!(".{}-", pack.id))
            .tempdir_in(&self.root)
            .map_err(|error| {
                font_error("font_filesystem", "create font staging directory", error)
            })?;
        let mut installed = BTreeMap::new();

        match pack.source {
            PackSource::Archive { url, sha256 } => {
                let archive = self.downloader.download(url)?;
                verify_hash(&archive, sha256)?;
                let mut archive = ZipArchive::new(Cursor::new(archive))
                    .map_err(|error| font_error("font_integrity", "open font archive", error))?;
                for file in pack.files {
                    let archive_path = file.archive_path.ok_or_else(|| {
                        AppError::new(
                            "font_integrity",
                            "The font catalog archive member is missing.",
                        )
                    })?;
                    let mut member = archive.by_name(archive_path).map_err(|error| {
                        font_error("font_integrity", "read font archive member", error)
                    })?;
                    let mut bytes = Vec::new();
                    member.read_to_end(&mut bytes).map_err(|error| {
                        font_error("font_integrity", "extract font archive member", error)
                    })?;
                    verify_hash(&bytes, file.sha256)?;
                    write_staged(staging.path(), file.output, &bytes)?;
                    installed.insert(file.output.to_owned(), hash(&bytes));
                }
            }
            PackSource::Files => {
                for file in pack.files {
                    let url = file.url.ok_or_else(|| {
                        AppError::new("font_integrity", "The font catalog file URL is missing.")
                    })?;
                    let bytes = self.downloader.download(url)?;
                    verify_hash(&bytes, file.sha256)?;
                    write_staged(staging.path(), file.output, &bytes)?;
                    installed.insert(file.output.to_owned(), hash(&bytes));
                }
            }
        }

        let manifest = InstalledManifest {
            id: pack.id.into(),
            version: pack.version.into(),
            files: installed,
        };
        fs::write(
            staging.path().join("manifest.json"),
            serde_json::to_vec_pretty(&manifest)
                .map_err(|error| font_error("font_integrity", "encode font manifest", error))?,
        )
        .map_err(|error| font_error("font_filesystem", "write font manifest", error))?;

        let destination = self.root.join(pack.id);
        if destination.exists() {
            fs::remove_dir_all(&destination).map_err(|error| {
                font_error("font_filesystem", "replace invalid font pack", error)
            })?;
        }
        fs::rename(staging.path(), &destination)
            .map_err(|error| font_error("font_filesystem", "activate font pack", error))?;
        let status = self.status(pack);
        if status.state != FontPackState::Installed {
            return Err(AppError::new(
                "font_integrity",
                "The installed font pack did not pass verification.",
            ));
        }
        Ok(status)
    }

    pub(crate) fn export_files(&self, pack_ids: &[String]) -> AppResult<Vec<(String, Vec<u8>)>> {
        let mut output = Vec::new();
        for pack_id in pack_ids.iter().collect::<BTreeSet<_>>() {
            let pack = find_pack(pack_id)?;
            let status = self.status(pack);
            if status.state != FontPackState::Installed {
                return Err(AppError::with_details(
                    "export_validation",
                    "A required portable font pack is missing or invalid.",
                    pack.id,
                ));
            }
            for file in pack.files {
                let path = self.root.join(pack.id).join(file.output);
                let bytes = fs::read(&path).map_err(|error| {
                    font_error("font_filesystem", "read installed font file", error)
                })?;
                output.push((format!("fonts/{}/{}", pack.id, file.output), bytes));
            }
        }
        Ok(output)
    }

    pub(crate) fn family(&self, pack_id: &str) -> AppResult<LatexFontFamily> {
        Ok(find_pack(pack_id)?.family)
    }

    #[cfg(test)]
    pub(crate) fn seeded_for_tests(root: PathBuf, pack_ids: &[&str]) -> Self {
        let manager = Self {
            root,
            downloader: Arc::new(TestDownloader),
            verification: VerificationMode::TestManifest,
        };
        fs::create_dir_all(&manager.root).expect("test font root");
        for pack_id in pack_ids {
            let pack = find_pack(pack_id).expect("test catalog pack");
            let directory = manager.root.join(pack.id);
            fs::create_dir_all(&directory).expect("test pack directory");
            let mut files = BTreeMap::new();
            for file in pack.files {
                let bytes = format!("test font fixture: {}/{}", pack.id, file.output).into_bytes();
                fs::write(directory.join(file.output), &bytes).expect("test font file");
                files.insert(file.output.to_owned(), hash(&bytes));
            }
            fs::write(
                directory.join("manifest.json"),
                serde_json::to_vec(&InstalledManifest {
                    id: pack.id.into(),
                    version: pack.version.into(),
                    files,
                })
                .expect("test manifest"),
            )
            .expect("write test manifest");
        }
        manager
    }

    fn status(&self, pack: &Pack) -> FontPackStatus {
        let directory = self.root.join(pack.id);
        if !directory.exists() {
            return pack_status(pack, FontPackState::Missing, 0);
        }
        let verified = self.verify_installed(pack);
        let bytes = if verified {
            pack.files
                .iter()
                .filter_map(|file| fs::metadata(directory.join(file.output)).ok())
                .map(|metadata| metadata.len())
                .sum()
        } else {
            0
        };
        pack_status(
            pack,
            if verified {
                FontPackState::Installed
            } else {
                FontPackState::Invalid
            },
            bytes,
        )
    }

    fn verify_installed(&self, pack: &Pack) -> bool {
        let directory = self.root.join(pack.id);
        let manifest = fs::read(directory.join("manifest.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<InstalledManifest>(&bytes).ok());
        let Some(manifest) = manifest else {
            return false;
        };
        if manifest.id != pack.id || manifest.version != pack.version {
            return false;
        }
        let expected = pack
            .files
            .iter()
            .map(|file| file.output)
            .collect::<BTreeSet<_>>();
        if manifest
            .files
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            != expected
        {
            return false;
        }
        pack.files.iter().all(|file| {
            let Some(manifest_hash) = manifest.files.get(file.output) else {
                return false;
            };
            let expected_hash = match self.verification {
                VerificationMode::Catalog => file.sha256,
                #[cfg(test)]
                VerificationMode::TestManifest => manifest_hash,
            };
            manifest_hash == expected_hash
                && fs::read(directory.join(file.output))
                    .is_ok_and(|bytes| hash(&bytes) == expected_hash)
        })
    }
}

#[cfg(test)]
struct TestDownloader;

#[cfg(test)]
impl Downloader for TestDownloader {
    fn download(&self, _url: &str) -> AppResult<Vec<u8>> {
        Err(AppError::new(
            "font_download",
            "Test font manager does not use the network.",
        ))
    }
}

fn write_staged(root: &Path, name: &str, bytes: &[u8]) -> AppResult<()> {
    let path = root.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| font_error("font_filesystem", "create font pack directory", error))?;
    }
    fs::write(path, bytes).map_err(|error| font_error("font_filesystem", "write font file", error))
}

fn verify_hash(bytes: &[u8], expected: &str) -> AppResult<()> {
    let actual = hash(bytes);
    if actual != expected {
        return Err(AppError::with_details(
            "font_integrity",
            "The downloaded font pack failed SHA-256 verification.",
            format!("expected={expected};actual={actual}"),
        ));
    }
    Ok(())
}

fn hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn pack_status(pack: &Pack, state: FontPackState, installed_bytes: u64) -> FontPackStatus {
    FontPackStatus {
        id: pack.id.into(),
        version: pack.version.into(),
        state,
        mandatory: pack.mandatory,
        installed_bytes,
    }
}

fn find_pack(id: &str) -> AppResult<&'static Pack> {
    catalog().iter().find(|pack| pack.id == id).ok_or_else(|| {
        AppError::with_details(
            "font_unknown",
            "The requested font pack is not in the bkuw catalog.",
            id,
        )
    })
}

fn font_error(code: &'static str, action: &str, error: impl std::fmt::Display) -> AppError {
    AppError::with_details(code, format!("Could not {action}."), error.to_string())
}

fn catalog() -> &'static [Pack] {
    &CATALOG
}

const TERMES_FILES: &[PackFile] = &[
    PackFile {
        output: "texgyretermes-regular.otf",
        archive_path: Some("fonts/opentype/public/tex-gyre/texgyretermes-regular.otf"),
        url: None,
        sha256: "cc3fe7c707b81428d23d54df3eadd9228a2bf6a4d43125d94df56f5f63134659",
    },
    PackFile {
        output: "texgyretermes-bold.otf",
        archive_path: Some("fonts/opentype/public/tex-gyre/texgyretermes-bold.otf"),
        url: None,
        sha256: "2fb3e952065fa153c7e4e64e04b98b9d79225739b6025aa3f0f0782d299ff61e",
    },
    PackFile {
        output: "texgyretermes-italic.otf",
        archive_path: Some("fonts/opentype/public/tex-gyre/texgyretermes-italic.otf"),
        url: None,
        sha256: "6dd103a1672e50568cd2f8a706ccd48443d44d7d073a59d2286f4e6f746575d6",
    },
    PackFile {
        output: "texgyretermes-bolditalic.otf",
        archive_path: Some("fonts/opentype/public/tex-gyre/texgyretermes-bolditalic.otf"),
        url: None,
        sha256: "1bf6af99cb0e26c12951317032d79b96ae009551e59ccf02a5b24f325ecfec87",
    },
    PackFile {
        output: "LICENSE.txt",
        archive_path: Some("doc/fonts/tex-gyre/GUST-FONT-LICENSE.txt"),
        url: None,
        sha256: "2bd69affc3da00715116f713f57eab9707e96daf3562ad0215987b15b9c16f73",
    },
];

const CHARIS_FILES: &[PackFile] = &[
    PackFile {
        output: "Charis-Regular.ttf",
        archive_path: Some("Charis-7.000/Charis-Regular.ttf"),
        url: None,
        sha256: "c03738834bd3a43c3e4a59b11878bd6ede3ee505998242ae649ed1f9cd2edcf6",
    },
    PackFile {
        output: "Charis-Bold.ttf",
        archive_path: Some("Charis-7.000/Charis-Bold.ttf"),
        url: None,
        sha256: "64e353674f294993fe979075e351afa5da367a8dd16a860e04eba103cb1cdb08",
    },
    PackFile {
        output: "Charis-Italic.ttf",
        archive_path: Some("Charis-7.000/Charis-Italic.ttf"),
        url: None,
        sha256: "f6e3e822b125aca03337f15cdd094663efaa784b67b766b9370d9f1bf5c2cb2e",
    },
    PackFile {
        output: "Charis-BoldItalic.ttf",
        archive_path: Some("Charis-7.000/Charis-BoldItalic.ttf"),
        url: None,
        sha256: "d832bcf31000994b07ab8f73b69127d12e764d8c3c67b7c90e58f1d740291d00",
    },
    PackFile {
        output: "LICENSE.txt",
        archive_path: Some("Charis-7.000/OFL.txt"),
        url: None,
        sha256: "07b1a63504f43e26b07a8017cd5803da86badb1bc1432470869ec48ad76958ee",
    },
];

const NOTO_LICENSE: PackFile = PackFile {
    output: "LICENSE.txt",
    archive_path: None,
    url: Some(
        "https://raw.githubusercontent.com/notofonts/notofonts.github.io/341cc991ffa33bb58fd0cb08728c6c6ac6c3b19a/LICENSE",
    ),
    sha256: "c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4",
};
const NOTO_CJK_LICENSE: PackFile = PackFile {
    output: "LICENSE.txt",
    archive_path: None,
    url: Some(
        "https://raw.githubusercontent.com/notofonts/noto-cjk/9b0f1436e455d902de067a2501422e5dc71ad16b/Serif/LICENSE",
    ),
    sha256: "6a73f9541c2de74158c0e7cf6b0a58ef774f5a780bf191f2d7ec9cc53efe2bf2",
};

const NOTO_SERIF_FILES: &[PackFile] = &[
    PackFile {
        output: "NotoSerif-Regular.ttf",
        archive_path: None,
        url: Some(
            "https://raw.githubusercontent.com/notofonts/notofonts.github.io/341cc991ffa33bb58fd0cb08728c6c6ac6c3b19a/fonts/NotoSerif/googlefonts/ttf/NotoSerif-Regular.ttf",
        ),
        sha256: "f182e1245d978506e7c64c048f992e7d2dfe5ca8bc851af55ad66da0b2214f7c",
    },
    PackFile {
        output: "NotoSerif-Bold.ttf",
        archive_path: None,
        url: Some(
            "https://raw.githubusercontent.com/notofonts/notofonts.github.io/341cc991ffa33bb58fd0cb08728c6c6ac6c3b19a/fonts/NotoSerif/googlefonts/ttf/NotoSerif-Bold.ttf",
        ),
        sha256: "c2995523c6c74add1a70e6e3a5622957b6431ccd337995bb20fce29ec4286d31",
    },
    NOTO_LICENSE,
];
const NOTO_CJK_FILES: &[PackFile] = &[
    PackFile {
        output: "NotoSerifCJKtc-Regular.otf",
        archive_path: None,
        url: Some(
            "https://raw.githubusercontent.com/notofonts/noto-cjk/9b0f1436e455d902de067a2501422e5dc71ad16b/Serif/OTF/TraditionalChinese/NotoSerifCJKtc-Regular.otf",
        ),
        sha256: "234301038e76e7c35c43113785024700c4e4fe7bdce1d1fbbc42fca7e6683798",
    },
    PackFile {
        output: "NotoSerifCJKtc-Bold.otf",
        archive_path: None,
        url: Some(
            "https://raw.githubusercontent.com/notofonts/noto-cjk/9b0f1436e455d902de067a2501422e5dc71ad16b/Serif/OTF/TraditionalChinese/NotoSerifCJKtc-Bold.otf",
        ),
        sha256: "a4441a76dbf56719600c5dcbd5b5e5a068a20944cc41c959487a657133576ee6",
    },
    NOTO_CJK_LICENSE,
];
const CHIRON_SUNG_HK_FILES: &[PackFile] = &[
    PackFile {
        output: "ChironSungHK-R.otf",
        archive_path: None,
        url: Some(
            "https://raw.githubusercontent.com/chiron-fonts/chiron-sung-hk/v1.024/STATIC_OTF/ChironSungHK-R.otf",
        ),
        sha256: "d53da9ffb3593a6dcce34f9e4a5d94369c7e619efa1b3d9a60717c7d72aa65d0",
    },
    PackFile {
        output: "ChironSungHK-B.otf",
        archive_path: None,
        url: Some(
            "https://raw.githubusercontent.com/chiron-fonts/chiron-sung-hk/v1.024/STATIC_OTF/ChironSungHK-B.otf",
        ),
        sha256: "bc3f911227c98ae45caf31147201c29dd3cf14fc2c84af72936f4ef6067b397d",
    },
    PackFile {
        output: "LICENSE.txt",
        archive_path: None,
        url: Some(
            "https://raw.githubusercontent.com/chiron-fonts/chiron-sung-hk/v1.024/LICENSE.md",
        ),
        sha256: "f610abd1c4f410c07fe99e6be924a500e577a378b182a73c691527e2e368c96f",
    },
];
const CHIRON_HEI_HK_FILES: &[PackFile] = &[
    PackFile {
        output: "ChironHeiHK-R.otf",
        archive_path: None,
        url: Some(
            "https://raw.githubusercontent.com/chiron-fonts/chiron-hei-hk/v2.609/STATIC_OTF/ChironHeiHK-R.otf",
        ),
        sha256: "72f68279a78a118b469bc683a1ab12364a5a3c244ee592da441d4a3cb0eda4b1",
    },
    PackFile {
        output: "ChironHeiHK-B.otf",
        archive_path: None,
        url: Some(
            "https://raw.githubusercontent.com/chiron-fonts/chiron-hei-hk/v2.609/STATIC_OTF/ChironHeiHK-B.otf",
        ),
        sha256: "51a6c39f8dd0a4522a7c5d366d77e9e52aebe4280d9b16ed95f03288c802e821",
    },
    PackFile {
        output: "LICENSE.txt",
        archive_path: None,
        url: Some("https://raw.githubusercontent.com/chiron-fonts/chiron-hei-hk/v2.609/LICENSE.md"),
        sha256: "8f6a49079a7accfffdff0026c868bbb5e62ada6a9b2554c28ae94349bdeab03f",
    },
];
const CATALOG: [Pack; 6] = [
    Pack {
        id: TERMES_PACK_ID,
        version: "2.004",
        mandatory: true,
        source: PackSource::Archive {
            url: "https://ctan.net/install/fonts/tex-gyre.tds.zip",
            sha256: "5981f5489e1f21ed9ec53f97a8fd7509cfb873ed6b8dfa2953c425bc9117616f",
        },
        files: TERMES_FILES,
        family: LatexFontFamily {
            regular: "texgyretermes-regular.otf",
            bold: Some("texgyretermes-bold.otf"),
            italic: Some("texgyretermes-italic.otf"),
            bold_italic: Some("texgyretermes-bolditalic.otf"),
        },
    },
    Pack {
        id: CHARIS_PACK_ID,
        version: "7.000",
        mandatory: false,
        source: PackSource::Archive {
            url: "https://github.com/silnrsi/font-charis/releases/download/v7.000/Charis-7.000.zip",
            sha256: "e3237b1303c5d31af8f59b1d1914886c5e873b77c71390e4742fb3bc1c187666",
        },
        files: CHARIS_FILES,
        family: LatexFontFamily {
            regular: "Charis-Regular.ttf",
            bold: Some("Charis-Bold.ttf"),
            italic: Some("Charis-Italic.ttf"),
            bold_italic: Some("Charis-BoldItalic.ttf"),
        },
    },
    Pack {
        id: NOTO_SERIF_PACK_ID,
        version: NOTO_COMMIT,
        mandatory: false,
        source: PackSource::Files,
        files: NOTO_SERIF_FILES,
        family: LatexFontFamily {
            regular: "NotoSerif-Regular.ttf",
            bold: Some("NotoSerif-Bold.ttf"),
            italic: None,
            bold_italic: None,
        },
    },
    Pack {
        id: NOTO_CJK_TC_PACK_ID,
        version: "2.003",
        mandatory: false,
        source: PackSource::Files,
        files: NOTO_CJK_FILES,
        family: LatexFontFamily {
            regular: "NotoSerifCJKtc-Regular.otf",
            bold: Some("NotoSerifCJKtc-Bold.otf"),
            italic: None,
            bold_italic: None,
        },
    },
    Pack {
        id: CHIRON_SUNG_HK_PACK_ID,
        version: "1.024",
        mandatory: false,
        source: PackSource::Files,
        files: CHIRON_SUNG_HK_FILES,
        family: LatexFontFamily {
            regular: "ChironSungHK-R.otf",
            bold: Some("ChironSungHK-B.otf"),
            italic: None,
            bold_italic: None,
        },
    },
    Pack {
        id: CHIRON_HEI_HK_PACK_ID,
        version: "2.609",
        mandatory: false,
        source: PackSource::Files,
        files: CHIRON_HEI_HK_FILES,
        family: LatexFontFamily {
            regular: "ChironHeiHK-R.otf",
            bold: Some("ChironHeiHK-B.otf"),
            italic: None,
            bold_italic: None,
        },
    },
];

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        sync::{Arc, Mutex},
    };

    use tempfile::tempdir;
    use zip::{ZipWriter, write::SimpleFileOptions};

    use super::*;

    #[derive(Default)]
    struct FakeDownloader {
        responses: Mutex<BTreeMap<String, Vec<u8>>>,
    }

    impl Downloader for FakeDownloader {
        fn download(&self, url: &str) -> AppResult<Vec<u8>> {
            self.responses
                .lock()
                .expect("responses")
                .get(url)
                .cloned()
                .ok_or_else(|| AppError::new("font_download", "missing fake response"))
        }
    }

    fn archive(files: &[(&str, &[u8])]) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(cursor);
        for (name, contents) in files {
            zip.start_file(*name, SimpleFileOptions::default())
                .expect("member");
            std::io::Write::write_all(&mut zip, contents).expect("contents");
        }
        zip.finish().expect("zip").into_inner()
    }

    #[test]
    fn mandatory_termes_pack_is_missing_until_an_integrity_checked_install() {
        let directory = tempdir().expect("temp directory");
        let manager = FontManager::with_downloader(
            directory.path().into(),
            Arc::new(FakeDownloader::default()),
        );
        let termes = manager
            .statuses()
            .into_iter()
            .find(|pack| pack.id == TERMES_PACK_ID)
            .expect("Termes catalog entry");
        assert!(termes.mandatory);
        assert_eq!(termes.state, FontPackState::Missing);
    }

    #[test]
    fn catalog_contains_only_the_supported_portable_font_packs() {
        let directory = tempdir().expect("temp directory");
        let manager = FontManager::with_downloader(
            directory.path().into(),
            Arc::new(FakeDownloader::default()),
        );
        assert_eq!(
            manager
                .statuses()
                .into_iter()
                .map(|pack| pack.id)
                .collect::<Vec<_>>(),
            vec![
                TERMES_PACK_ID,
                CHARIS_PACK_ID,
                NOTO_SERIF_PACK_ID,
                NOTO_CJK_TC_PACK_ID,
                CHIRON_SUNG_HK_PACK_ID,
                CHIRON_HEI_HK_PACK_ID,
            ]
        );
    }

    #[test]
    fn a_checksum_mismatch_is_rejected_without_installing_partial_files() {
        let directory = tempdir().expect("temp directory");
        let downloader = Arc::new(FakeDownloader::default());
        downloader.responses.lock().expect("responses").insert(
            "https://ctan.net/install/fonts/tex-gyre.tds.zip".into(),
            archive(&[("unexpected", b"tampered")]),
        );
        let manager = FontManager::with_downloader(directory.path().into(), downloader);
        let error = manager
            .install(TERMES_PACK_ID)
            .expect_err("checksum must fail");
        assert_eq!(error.code, "font_integrity");
        assert_eq!(
            manager.status_for(TERMES_PACK_ID).expect("status").state,
            FontPackState::Missing
        );
    }

    #[test]
    fn exported_pack_files_are_namespaced_and_include_the_license() {
        let directory = tempdir().expect("temp directory");
        let manager = FontManager::seeded_for_tests(directory.path().into(), &[TERMES_PACK_ID]);
        let names = manager
            .export_files(&[TERMES_PACK_ID.into()])
            .expect("export files")
            .into_iter()
            .map(|(name, _)| name)
            .collect::<Vec<_>>();
        assert!(names.contains(&"fonts/tex-gyre-termes/texgyretermes-regular.otf".into()));
        assert!(names.contains(&"fonts/tex-gyre-termes/LICENSE.txt".into()));
        assert!(!names.iter().any(|name| name.ends_with("manifest.json")));
    }

    #[test]
    fn both_chiron_packs_export_static_faces_and_separate_licenses() {
        let directory = tempdir().expect("temp directory");
        let manager = FontManager::seeded_for_tests(
            directory.path().into(),
            &[CHIRON_SUNG_HK_PACK_ID, CHIRON_HEI_HK_PACK_ID],
        );
        let names = manager
            .export_files(&[CHIRON_SUNG_HK_PACK_ID.into(), CHIRON_HEI_HK_PACK_ID.into()])
            .expect("export files")
            .into_iter()
            .map(|(name, _)| name)
            .collect::<Vec<_>>();
        for expected in [
            "fonts/chiron-sung-hk/ChironSungHK-R.otf",
            "fonts/chiron-sung-hk/ChironSungHK-B.otf",
            "fonts/chiron-sung-hk/LICENSE.txt",
            "fonts/chiron-hei-hk/ChironHeiHK-R.otf",
            "fonts/chiron-hei-hk/ChironHeiHK-B.otf",
            "fonts/chiron-hei-hk/LICENSE.txt",
        ] {
            assert!(
                names.iter().any(|name| name == expected),
                "missing {expected}"
            );
        }
    }

    #[test]
    fn changing_an_installed_file_marks_the_pack_invalid() {
        let directory = tempdir().expect("temp directory");
        let manager = FontManager::seeded_for_tests(directory.path().into(), &[TERMES_PACK_ID]);
        let pack = directory.path().join(TERMES_PACK_ID);
        fs::write(pack.join("texgyretermes-regular.otf"), b"corrupted").expect("corrupt");
        assert_eq!(
            manager.status_for(TERMES_PACK_ID).expect("status").state,
            FontPackState::Invalid
        );
    }
}
