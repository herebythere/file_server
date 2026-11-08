pub struct FileDetails {
    pub is_file: bool,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub file_name: String,
    pub url_path: String,
    pub size: u64,
    pub created_at: Option<u128>,
    pub accessed_at: Option<u128>,
    pub modified_at: Option<u128>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FileDetailsJson {
    is_file: bool,
    is_dir: bool,
    is_symlink: bool,
    file_name: String,
    url_path: String,
    size: String,
    created_at: Option<String>,
    accessed_at: Option<String>,
    modified_at: Option<String>,
}

impl FileDetailsJson {
    pub fn as_file_details(&self) -> Result<FileDetails, ParseIntError> {
        let size: u64 = match self.size.parse() {
            Ok(num) => num,
            Err(e) => return Err(e),
        };
        let created_at = match get_optional_number(&self.created_at) {
            Ok(num) => num,
            Err(e) => return Err(e),
        };
        let accessed_at = match get_optional_number(&self.accessed_at) {
            Ok(num) => num,
            Err(e) => return Err(e),
        };
        let modified_at = match get_optional_number(&self.modified_at) {
            Ok(num) => num,
            Err(e) => return Err(e),
        };

        Ok(FileDetails {
            is_file: self.is_file,
            is_dir: self.is_dir,
            is_symlink: self.is_symlink,
            file_name: self.file_name.clone(),
            url_path: self.url_path.clone(),
            size,
            created_at,
            accessed_at,
            modified_at,
        })
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct EntryDetailsJson {
    details: FileDetailsJson,
    entries: Vec<FileDetailsJson>,
}