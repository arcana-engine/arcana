use std::{fs::File, path::Path};

use treasury_import::{ensure_dependencies, AssetId, Dependencies, ImportError, Importer, Sources};

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Key {
    AssetId(AssetId),
    Path(Box<str>),
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColliderKind {
    Wall,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Tile {
    #[serde(default)]
    pub collider: Option<ColliderKind>,
    pub texture: Option<Key>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct TileSet {
    pub tiles: Vec<Tile>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct TileMap {
    pub set: Key,
    pub cell_size: f32,
    pub width: usize,
    pub cells: Vec<usize>,
}

pub struct TileSetImporter;

impl Importer for TileSetImporter {
    fn import(
        &self,
        source: &Path,
        output: &Path,
        _sources: &impl Sources,
        dependencies: &impl Dependencies,
    ) -> Result<(), ImportError> {
        let source_file = File::open(source).map_err(|err| ImportError::Other {
            reason: format!(
                "Failed to open tile set source '{}', {:#}",
                source.display(),
                err
            ),
        })?;

        let mut set: TileSet =
            serde_json::from_reader(source_file).map_err(|err| ImportError::Other {
                reason: format!(
                    "Failed to deserialize tile set from '{}', {:#}",
                    source.display(),
                    err
                ),
            })?;

        let mut missing_deps = Vec::new();

        for tile in &mut set.tiles {
            if let Some(Key::Path(path)) = &tile.texture {
                match dependencies.get_or_append(path, "qoi", &mut missing_deps) {
                    Err(err) => {
                        return Err(ImportError::Other {
                            reason: format!("Failed to fetch tile texture '{}'. {:#}", path, err),
                        })
                    }
                    Ok(None) => {}
                    Ok(Some(id)) => {
                        tile.texture = Some(Key::AssetId(id));
                    }
                }
            }
        }

        ensure_dependencies(missing_deps)?;

        let output_file = File::create(output).map_err(|err| ImportError::Other {
            reason: format!(
                "Failed to create tile set artifact '{}', {:#}",
                output.display(),
                err
            ),
        })?;

        serde_json::to_writer(output_file, &set).map_err(|err| ImportError::Other {
            reason: format!(
                "Failed to serialize tile set to '{}', {:#}",
                output.display(),
                err
            ),
        })?;

        Ok(())
    }
}

pub struct TileMapImporter;

impl Importer for TileMapImporter {
    fn import(
        &self,
        source: &Path,
        output: &Path,
        _sources: &impl Sources,
        dependencies: &impl Dependencies,
    ) -> Result<(), ImportError> {
        let source_file = File::open(source).map_err(|err| ImportError::Other {
            reason: format!(
                "Failed to open tile map source '{}', {:#}",
                source.display(),
                err
            ),
        })?;

        let mut map: TileMap =
            serde_json::from_reader(source_file).map_err(|err| ImportError::Other {
                reason: format!(
                    "Failed to deserialize tile map from '{}', {:#}",
                    source.display(),
                    err
                ),
            })?;

        let mut missing_deps = Vec::new();

        if let Key::Path(path) = &map.set {
            match dependencies.get_or_append(path, "arcana.tileset", &mut missing_deps) {
                Err(err) => {
                    return Err(ImportError::Other {
                        reason: format!("Failed to fetch tile texture '{}'. {:#}", path, err),
                    })
                }
                Ok(None) => {}
                Ok(Some(id)) => {
                    map.set = Key::AssetId(id);
                }
            }
        }

        ensure_dependencies(missing_deps)?;

        let output_file = File::create(output).map_err(|err| ImportError::Other {
            reason: format!(
                "Failed to create tile map artifact '{}', {:#}",
                output.display(),
                err
            ),
        })?;

        serde_json::to_writer(output_file, &map).map_err(|err| ImportError::Other {
            reason: format!(
                "Failed to serialize tile map to '{}', {:#}",
                output.display(),
                err
            ),
        })?;

        Ok(())
    }
}
