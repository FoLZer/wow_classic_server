use std::{collections::HashMap, io::BufRead, num::ParseIntError, str::FromStr};

#[derive(Debug, Clone)]
pub struct DBDefinition {
    pub column_definitions: HashMap<String, ColumnDefinition>,
    pub version_definitions: Vec<VersionDefinitions>,
}

impl DBDefinition {
    #[allow(clippy::field_reassign_with_default)]
    pub fn from_lines<T: BufRead>(lines: &mut std::io::Lines<T>) -> Result<Self, ParseError> {
        {
            let Some(line) = lines.next() else {
                return Err(ParseError::IoError(std::io::Error::from(
                    std::io::ErrorKind::UnexpectedEof,
                )));
            };
            let line = line?;
            if !line.starts_with("COLUMNS") {
                return Err(ParseError::ExpectedColumnDef);
            }
        }

        let mut column_definition_dictionary = HashMap::new();

        let lines = lines.collect::<Result<Vec<_>, _>>()?;
        let len = lines.len();
        let mut lines = lines.into_iter();

        for line in lines.by_ref() {
            if line.trim().is_empty() {
                break;
            }
            let mut column_definition = ColumnDefinition::default();
            if !line.contains(" ") {
                return Err(ParseError::NoSpaceTypeColumn);
            }
            let ty = &line[0..line.chars().position(|c| [' ', '<'].contains(&c)).unwrap()];
            column_definition.ty = match ty {
                "uint" => DBCType::Uint,
                "int" => DBCType::Int,
                "float" => DBCType::Float,
                "string" => DBCType::String,
                "locstring" => DBCType::LocString,
                _ => return Err(ParseError::InvalidType),
            };
            if line.starts_with(&format!("{}<", ty)) {
                let foreign_key = line[line.chars().position(|c| c == '<').unwrap() + 1
                    ..line.chars().position(|c| c == '>').unwrap()]
                    .split("::")
                    .collect::<Vec<&str>>();
                if foreign_key.len() != 2 {
                    return Err(ParseError::InvalidForeignKeyLength);
                } else {
                    column_definition.foreign_table = foreign_key[0].to_owned();
                    column_definition.foreign_column = foreign_key[1].to_owned();
                }
            }

            let mut name = if line.chars().filter(|c| *c == ' ').count() == 1 {
                &line[line.find(' ').unwrap() + 1..]
            } else {
                let mut chars = line.chars();
                let start = chars.position(|c| c == ' ').unwrap() + 1;
                chars.next().unwrap();
                let end = start + 1 + chars.position(|c| c == ' ').unwrap();
                &line[start..end]
            };

            if name.ends_with('?') {
                column_definition.verified = false;
                name = &name[..name.len() - 1];
            } else {
                column_definition.verified = true;
            }

            if line.contains("//") {
                column_definition.comment = line[line.find("//").unwrap() + 2..].trim().to_owned();
            }

            if column_definition_dictionary.contains_key(name) {
                println!(
                    "Collision with existing column name while adding new column name! Skipping..."
                );
            } else {
                column_definition_dictionary.insert(name.to_owned(), column_definition);
            }
        }

        let mut version_definitions = Vec::new();

        let mut definitions = Vec::new();
        let mut layout_hashes = Vec::new();
        let mut comment = "".to_owned();
        let mut builds = Vec::new();
        let mut build_ranges = Vec::new();

        for (i, mut line) in lines.enumerate() {
            if line.trim().is_empty() {
                if !builds.is_empty() || !build_ranges.is_empty() || !layout_hashes.is_empty() {
                    version_definitions.push(VersionDefinitions {
                        builds: builds.clone(),
                        build_ranges: build_ranges.clone(),
                        layout_hashes: layout_hashes.clone(),
                        comment: comment.to_owned(),
                        definitions: definitions.clone(),
                    })
                } else if !definitions.is_empty() || !comment.trim().is_empty() {
                    return Err(ParseError::NoBuildOrLayout);
                }

                definitions.clear();
                layout_hashes.clear();
                comment = "".to_owned();
                builds.clear();
                build_ranges.clear();
            }

            if line.starts_with("LAYOUT") {
                let split_layout_hashes = line[7..].split(", ").map(|s| s.to_owned());
                layout_hashes.extend(split_layout_hashes)
            }

            if line.starts_with("BUILD") {
                let split_builds = line[6..].split(", ");
                for split_build in split_builds {
                    if split_build.contains('-') {
                        let mut split_range = split_build.split('-');
                        build_ranges.push(BuildRange {
                            min_build: Build::from_str(split_range.next().unwrap())?,
                            max_build: Build::from_str(split_range.next().unwrap())?,
                        })
                    } else {
                        let build = Build::from_str(split_build)?;
                        builds.push(build);
                    }
                }
            }

            if let Some(line) = line.strip_prefix("COMMENT") {
                comment = line.trim().to_owned();
            }

            if !line.starts_with("LAYOUT")
                && !line.starts_with("BUILD")
                && !line.starts_with("COMMENT")
                && !line.trim().is_empty()
            {
                let mut definition = Definition::default();
                definition.is_non_inline = false;
                if line.contains("$") {
                    let mut c = line.chars();
                    let annotation_start = c.position(|c| c == '$').unwrap();
                    c.next().unwrap();
                    let annotation_end = annotation_start + 1 + c.position(|c| c == '$').unwrap();

                    let annotations = line[annotation_start + 1..annotation_end]
                        .split(',')
                        .collect::<Vec<_>>();
                    if annotations.contains(&"id") {
                        definition.is_id = true;
                    }

                    if annotations.contains(&"noninline") {
                        definition.is_non_inline = true;
                    }

                    if annotations.contains(&"relation") {
                        definition.is_relation = true;
                    }

                    line = line
                        .chars()
                        .take(annotation_start)
                        .chain(line.chars().skip(annotation_end + 2))
                        .collect();
                }

                if line.contains('<') {
                    let size = &line[line.find('<').unwrap() + 1..line.find('>').unwrap()];
                    definition.size = if let Some(size) = size.strip_prefix('u') {
                        definition.is_signed = false;
                        size.parse()?
                    } else {
                        definition.is_signed = true;
                        size.parse()?
                    };
                    line = line
                        .chars()
                        .take(line.find('<').unwrap())
                        .chain(line.chars().skip(line.find('>').unwrap() + 1))
                        .collect();
                }

                if line.contains('[') {
                    definition.arr_length =
                        line[line.find('[').unwrap() + 1..line.find(']').unwrap()].parse()?;
                    line = line
                        .chars()
                        .take(line.find('[').unwrap())
                        .chain(line.chars().skip(line.find(']').unwrap() + 1))
                        .collect();
                }

                if line.contains("//") {
                    definition.comment = line[line.find("//").unwrap() + 2..].trim().to_owned();
                    line = line[..line.find("//").unwrap()].trim().to_owned();
                }

                definition.name = line;

                if !column_definition_dictionary.contains_key(&definition.name) {
                    return Err(ParseError::KeyNotFound);
                } else if column_definition_dictionary[&definition.name].ty == DBCType::Uint {
                    definition.is_signed = false;
                }

                definitions.push(definition);
            }

            if len == (i + 1) {
                if !builds.is_empty() || !build_ranges.is_empty() || !layout_hashes.is_empty() {
                    version_definitions.push(VersionDefinitions {
                        builds: builds.clone(),
                        build_ranges: build_ranges.clone(),
                        layout_hashes: layout_hashes.clone(),
                        comment: comment.to_owned(),
                        definitions: definitions.clone(),
                    });
                } else if !definitions.is_empty() || !comment.trim().is_empty() {
                    return Err(ParseError::NoBuildOrLayout);
                }
            }
        }

        Ok(Self {
            column_definitions: column_definition_dictionary,
            version_definitions,
        })
    }
}

#[derive(Debug)]
pub enum ParseError {
    IoError(std::io::Error),
    ParseIntError(ParseIntError),
    ExpectedColumnDef,
    NoSpaceTypeColumn,
    InvalidType,
    InvalidForeignKeyLength,
    NoBuildOrLayout,
    KeyNotFound,
}

impl From<std::io::Error> for ParseError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

impl From<ParseIntError> for ParseError {
    fn from(value: ParseIntError) -> Self {
        Self::ParseIntError(value)
    }
}

#[derive(Debug, Clone)]
pub struct ColumnDefinition {
    pub ty: DBCType,
    pub foreign_table: String,
    pub foreign_column: String,
    pub verified: bool,
    pub comment: String,
}

impl Default for ColumnDefinition {
    fn default() -> Self {
        Self {
            ty: DBCType::Uint,
            foreign_table: Default::default(),
            foreign_column: Default::default(),
            verified: Default::default(),
            comment: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VersionDefinitions {
    pub builds: Vec<Build>,
    pub build_ranges: Vec<BuildRange>,
    pub layout_hashes: Vec<String>,
    pub comment: String,
    pub definitions: Vec<Definition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Build {
    pub expansion: i16,
    pub major: i16,
    pub minor: i16,
    pub build: u32,
}

impl FromStr for Build {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split('.');

        Ok(Self {
            expansion: split.next().unwrap().parse()?,
            major: split.next().unwrap().parse()?,
            minor: split.next().unwrap().parse()?,
            build: split.next().unwrap().parse()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct BuildRange {
    pub min_build: Build,
    pub max_build: Build,
}

impl BuildRange {
    pub fn contains(&self, other: &Build) -> bool {
        other.expansion >= self.min_build.expansion
            && other.expansion <= self.max_build.expansion
            && other.major >= self.min_build.major
            && other.major <= self.max_build.major
            && other.build >= self.min_build.build
            && other.build <= self.max_build.build
    }
}

#[derive(Clone, Default, Debug)]
pub struct Definition {
    pub size: i32,
    pub arr_length: i32,
    pub name: String,
    pub is_id: bool,
    pub is_relation: bool,
    pub is_non_inline: bool,
    pub is_signed: bool,
    pub comment: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DBCType {
    Uint,
    Int,
    Float,
    String,
    LocString,
}
