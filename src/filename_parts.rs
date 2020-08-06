const FILENAME_NUM_DIGITS: usize = 3;

/// Struct representing a filename that can be split, modified, and
/// merged back into a filename string
#[derive(PartialEq, Debug)]
pub struct FilenameParts {
    /// From the beginning of the filename to the final dot before the extension
    pub stem: String,

    /// The zero-padded collision-resolving number
    pub num: Option<usize>,

    /// The file extension, not including the dot
    pub ext: Option<String>,
}

impl FilenameParts {
    pub fn merge(&self) -> String {
        format!(
            "{}{}{}",
            self.stem,
            match self.num {
                // Format the collision-resolving number of a filename to a
                // zero-padded string
                Some(num) => format!("_{:0width$}", num, width = FILENAME_NUM_DIGITS),
                None => "".to_string(),
            },
            match &self.ext {
                Some(ext) => format!(".{}", ext),
                None => "".to_string(),
            }
        )
    }

    pub fn from_filename(filename: &str) -> Self {
        let mut it = filename.rsplitn(2, '.');
        let ext = it.next().expect("tried to split empty filename");
        let maybe_stem_num = it.next();

        // Set the stem-num combination to the extension if the iterator
        // said it was `None`. This is such that only the content after
        // the final dot is considered the extension, but extension-less
        // files are properly handled.
        let (stem_num, ext) = match maybe_stem_num {
            Some(stem_num) => (stem_num, Some(ext.to_string())),
            None => (ext, None),
        };

        // Hack to get an iterator over the last `FILENAME_NUM_DIGITS + 1`
        // characters of the stem-num combination. For files that have the
        // collision-resolving number, this is that prefixed with an
        // underscore.
        let num_it = stem_num
            .chars()
            .rev()
            .take(FILENAME_NUM_DIGITS + 1)
            .collect::<Vec<_>>();
        let mut num_it = num_it.iter().rev();

        // Determine if the filename has a collision-resolving number and
        // parse it
        let num = if num_it.next() == Some(&'_') && num_it.len() == FILENAME_NUM_DIGITS {
            num_it.collect::<String>().parse::<usize>().ok()
        } else {
            None
        };

        // Split the stem from the stem-num combination
        let stem = if num.is_some() {
            stem_num
                .chars()
                .take(stem_num.len() - FILENAME_NUM_DIGITS - 1)
                .collect()
        } else {
            stem_num.to_string()
        };

        Self { stem, num, ext }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_filename() {
        assert_eq!(
            FilenameParts::from_filename("a"),
            FilenameParts {
                stem: "a".to_string(),
                num: None,
                ext: None,
            }
        );
        assert_eq!(
            FilenameParts::from_filename("a."),
            FilenameParts {
                stem: "a".to_string(),
                num: None,
                ext: Some("".to_string()),
            }
        );
        assert_eq!(
            FilenameParts::from_filename(".a"),
            FilenameParts {
                stem: "".to_string(),
                num: None,
                ext: Some("a".to_string()),
            }
        );
        assert_eq!(
            FilenameParts::from_filename("a_0000"),
            FilenameParts {
                stem: "a_0000".to_string(),
                num: None,
                ext: None,
            }
        );
        assert_eq!(
            FilenameParts::from_filename("a_137"),
            FilenameParts {
                stem: "a".to_string(),
                num: Some(137),
                ext: None,
            }
        );
        assert_eq!(
            FilenameParts::from_filename("a_000.txt"),
            FilenameParts {
                stem: "a".to_string(),
                num: Some(0),
                ext: Some("txt".to_string()),
            }
        );
        assert_eq!(
            FilenameParts::from_filename("a____000.txt"),
            FilenameParts {
                stem: "a___".to_string(),
                num: Some(0),
                ext: Some("txt".to_string()),
            }
        );
        assert_eq!(
            FilenameParts::from_filename(".x._._._222.txt"),
            FilenameParts {
                stem: ".x._._.".to_string(),
                num: Some(222),
                ext: Some("txt".to_string()),
            }
        );
    }

    #[test]
    fn merge() {
        assert_eq!(
            "a",
            FilenameParts {
                stem: "a".to_string(),
                num: None,
                ext: None,
            }
            .merge()
        );
        assert_eq!(
            "a.",
            FilenameParts {
                stem: "a".to_string(),
                num: None,
                ext: Some("".to_string()),
            }
            .merge()
        );
        assert_eq!(
            ".a",
            FilenameParts {
                stem: "".to_string(),
                num: None,
                ext: Some("a".to_string()),
            }
            .merge()
        );
        assert_eq!(
            "a_0000",
            FilenameParts {
                stem: "a_0000".to_string(),
                num: None,
                ext: None,
            }
            .merge()
        );
        assert_eq!(
            "a_137",
            FilenameParts {
                stem: "a".to_string(),
                num: Some(137),
                ext: None,
            }
            .merge()
        );
        assert_eq!(
            "a_000.txt",
            FilenameParts {
                stem: "a".to_string(),
                num: Some(0),
                ext: Some("txt".to_string()),
            }
            .merge()
        );
        assert_eq!(
            "a____000.txt",
            FilenameParts {
                stem: "a___".to_string(),
                num: Some(0),
                ext: Some("txt".to_string()),
            }
            .merge()
        );
        assert_eq!(
            ".x._._._222.txt",
            FilenameParts {
                stem: ".x._._.".to_string(),
                num: Some(222),
                ext: Some("txt".to_string()),
            }
            .merge()
        );
    }
}
