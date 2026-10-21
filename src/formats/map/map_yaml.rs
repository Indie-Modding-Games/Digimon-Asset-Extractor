use crate::error::{DwExtractError, Result};
use crate::formats::map::{MapFile, MapSetup, MapSprite, MapObjects, MapEntities, MapNpc, MapWarps, MapCollision, MapImageSection, MapObjectInstance, MapReadOptions};
use serde::{Serialize, Serializer};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct MapYamlFile {
    pub setup: MapYamlSetup,
    pub images: Vec<MapYamlImage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub objects: Option<MapYamlObjects>,
    pub entities: MapYamlEntities,
    pub collision: MapYamlCollision,
}

#[derive(Debug, Serialize)]
pub struct MapYamlSetup {
    pub viewpoint: [i32; 3],
    pub reference: [i32; 3],
    pub lights: [MapYamlLight; 3],
    pub ambient: [i32; 3],
    pub projection: i32,
    pub region_a: [[i32; 4]; 2],
    pub width: i32,
    pub height: i32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tiles: Vec<i32>,
}

#[derive(Debug, Serialize)]
pub struct MapYamlLight {
    pub direction: [i32; 3],
    pub color: [i32; 3],
}

#[derive(Debug, Serialize)]
pub struct MapYamlImage {
    pub bits_per_pixel: u8,
    pub offset: usize,
    pub size: usize,
}

#[derive(Debug, Serialize)]
pub struct MapYamlObjects {
    pub sprites: Vec<MapYamlSprite>,
    pub instances: Vec<MapYamlInstance>,
}

#[derive(Debug, Serialize)]
pub struct MapYamlSprite {
    pub tex_x: i16,
    pub tex_y: i16,
    pub width: i16,
    pub height: i16,
    pub position: [i16; 3],
    pub clut: i16,
    pub transparency: i16,
}

#[derive(Debug, Serialize)]
pub struct MapYamlInstance {
    pub anim_sprites: [i16; 8],
    pub anim_times: [i16; 8],
    pub position: [i16; 2],
    pub flag: i16,
}

#[derive(Debug, Serialize)]
pub struct MapYamlEntities {
    pub warps: MapYamlWarps,
    pub digimon_count: i16,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub npcs: Vec<MapYamlNpc>,
}

#[derive(Debug, Serialize)]
pub struct MapYamlWarps {
    pub spawn: [[i16; 3]; 10],
    pub rotation: [i16; 10],
    pub target_map: [i16; 10],
    pub target_exit: [i16; 10],
}

#[derive(Debug, Serialize)]
pub struct MapYamlNpc {
    pub type_id: i16,
    pub follow_mode: i16,
    pub position: [i16; 3],
    pub rotation: [i16; 3],
    pub tracking_range: i16,
    pub script_id: i16,
    pub stats: MapYamlNpcStats,
    pub ai: MapYamlNpcAi,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub waypoints: Vec<[i16; 3]>,
}

#[derive(Debug, Serialize)]
pub struct MapYamlNpcStats {
    pub hp: i16,
    pub mp: i16,
    pub current_hp: i16,
    pub current_mp: i16,
    pub off: i16,
    pub def: i16,
    pub speed: i16,
    pub brain: i16,
    pub bits: i16,
    pub charge_mode: i16,
    pub moves: [i16; 4],
    pub moves_prio: [i16; 4],
    pub flee: [i16; 3],
}

#[derive(Debug, Serialize)]
pub struct MapYamlNpcAi {
    pub sections: [i16; 8],
}

#[derive(Debug, Serialize)]
pub struct MapYamlCollision {
    pub width: usize,
    pub height: usize,
    #[serde(serialize_with = "serialize_collision")]
    pub data: Vec<u8>,
}

fn serialize_collision<S>(data: &Vec<u8>, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use serde::ser::SerializeSeq;
    let mut seq = serializer.serialize_seq(Some(data.len()))?;
    for &byte in data {
        let blocked = byte & 0x80 != 0;
        let trigger = byte & 0x7F;
        seq.serialize_element(&serde_json::json!({
            "blocked": blocked,
            "trigger": trigger,
        }))?;
    }
    seq.end()
}

impl From<&MapSetup> for MapYamlSetup {
    fn from(s: &MapSetup) -> Self {
        Self {
            viewpoint: [s.vpx, s.vpy, s.vpz],
            reference: [s.vrx, s.vry, s.vrz],
            lights: s.lights.clone().map(|l| MapYamlLight {
                direction: [l.vx, l.vy, l.vz],
                color: [l.r, l.g, l.b],
            }),
            ambient: [s.ambient_r, s.ambient_g, s.ambient_b],
            projection: s.projection,
            region_a: [s.region_a, s.region_b],
            width: s.width,
            height: s.height,
            tiles: s.tiles.clone(),
        }
    }
}

impl From<&MapImageSection> for MapYamlImage {
    fn from(i: &MapImageSection) -> Self {
        Self { bits_per_pixel: i.bits_per_pixel, offset: i.offset, size: i.size }
    }
}

impl From<&MapSprite> for MapYamlSprite {
    fn from(s: &MapSprite) -> Self {
        Self {
            tex_x: s.tex_x, tex_y: s.tex_y,
            width: s.width, height: s.height,
            position: [s.some_x, s.some_y, s.some_z],
            clut: s.clut, transparency: s.transparency,
        }
    }
}

impl From<&MapObjectInstance> for MapYamlInstance {
    fn from(i: &MapObjectInstance) -> Self {
        Self {
            anim_sprites: i.anim_sprites,
            anim_times: i.anim_times,
            position: [i.x, i.y],
            flag: i.flag,
        }
    }
}

impl From<&MapObjects> for MapYamlObjects {
    fn from(o: &MapObjects) -> Self {
        Self {
            sprites: o.sprites.iter().map(|s| s.into()).collect(),
            instances: o.instances.iter().map(|i| i.into()).collect(),
        }
    }
}

impl From<&MapWarps> for MapYamlWarps {
    fn from(w: &MapWarps) -> Self {
        let mut spawn = [[0i16; 3]; 10];
        for i in 0..10 {
            spawn[i] = [w.spawn_x[i], w.spawn_y[i], w.spawn_z[i]];
        }
        Self {
            spawn,
            rotation: w.rotation,
            target_map: w.target_map,
            target_exit: w.target_exit,
        }
    }
}

impl From<&MapNpc> for MapYamlNpc {
    fn from(n: &MapNpc) -> Self {
        Self {
            type_id: n.type_id,
            follow_mode: n.follow_mode,
            position: [n.pos_x, n.pos_y, n.pos_z],
            rotation: [n.rot_x, n.rot_y, n.rot_z],
            tracking_range: n.tracking_range,
            script_id: n.script_id,
            stats: MapYamlNpcStats {
                hp: n.hp, mp: n.mp,
                current_hp: n.current_hp, current_mp: n.current_mp,
                off: n.off, def: n.def,
                speed: n.speed, brain: n.brain,
                bits: n.bits, charge_mode: n.charge_mode,
                moves: n.moves, moves_prio: n.moves_prio,
                flee: [n.flee_x, n.flee_y, n.flee_z],
            },
            ai: MapYamlNpcAi { sections: n.ai_sections },
            waypoints: n.waypoints.iter().map(|w| [w.x, w.y, w.z]).collect(),
        }
    }
}

impl From<&MapEntities> for MapYamlEntities {
    fn from(e: &MapEntities) -> Self {
        Self {
            warps: (&e.warps).into(),
            digimon_count: e.digimon_count,
            npcs: e.npcs.iter().map(|n| n.into()).collect(),
        }
    }
}

impl From<&MapCollision> for MapYamlCollision {
    fn from(c: &MapCollision) -> Self {
        Self {
            width: crate::formats::map::COLLISION_SIZE,
            height: crate::formats::map::COLLISION_SIZE,
            data: c.data.clone(),
        }
    }
}

pub fn map_file_to_yaml(map: &MapFile) -> MapYamlFile {
    MapYamlFile {
        setup: (&map.setup).into(),
        images: map.images.iter().map(|i| i.into()).collect(),
        objects: map.objects.as_ref().map(|o| o.into()),
        entities: (&map.entities).into(),
        collision: (&map.collision).into(),
    }
}

pub fn write_map_yaml<P: AsRef<Path>>(path: P, map: &MapFile) -> Result<()> {
    let yaml = map_file_to_yaml(map);
    let yaml_str = serde_yaml::to_string(&yaml)
        .map_err(|e| DwExtractError::Yaml(e))?;
    std::fs::write(path, yaml_str)?;
    Ok(())
}

pub fn export_map_obj<P: AsRef<Path>>(map: &MapFile, output_dir: P, extract_textures: bool) -> Result<()> {
    use crate::formats::tim::TimHeader;
    use std::fs;
    
    fs::create_dir_all(&output_dir)?;
    
    let mut obj = String::new();
    obj.push_str("# Digimon World Map Export\n");
    obj.push_str(&format!("# Map: {}x{} tiles\n", map.setup.width, map.setup.height));
    obj.push_str("\n");
    
    // Export ground plane from collision data
    let mut vertex_id = 1;
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    
    let tile_size = 100.0; // collision cell size in world units
    
    for y in 0..map.setup.height as usize {
        for x in 0..map.setup.width as usize {
            let tile_idx = map.setup.tiles[y * map.setup.width as usize + x];
            if tile_idx >= 0 {
                let blocked = map.collision.is_blocked(x, y);
                let h = if blocked { 10.0 } else { 0.0 };
                
                let x0 = x as f32 * tile_size;
                let y0 = y as f32 * tile_size;
                let x1 = (x + 1) as f32 * tile_size;
                let y1 = (y + 1) as f32 * tile_size;
                
                vertices.push((x0, h, y0));
                vertices.push((x1, h, y0));
                vertices.push((x1, h, y1));
                vertices.push((x0, h, y1));
                
                let base = vertex_id;
                faces.push([base, base + 1, base + 2]);
                faces.push([base, base + 2, base + 3]);
                vertex_id += 4;
}
    }
}
        
        for v in &vertices {
            obj.push_str(&format!("v {} {} {}\n", v.0, v.1, v.2));
        }
        
        obj.push_str("\n");
        
        for f in &faces {
            obj.push_str(&format!("f {} {} {}\n", f[0], f[1], f[2]));
        }
        
        let obj_path = output_dir.as_ref().join("map_ground.obj");
        fs::write(&obj_path, obj)?;
        println!("Exported OBJ: {}", obj_path.display());
        
        // Export entities as separate OBJ files
        export_entities_obj(map, &output_dir)?;
        
        if extract_textures {
            export_map_textures(map, &output_dir)?;
        }
        
        Ok(())
    }

fn export_entities_obj<P: AsRef<Path>>(map: &MapFile, output_dir: P) -> Result<()> {
    use std::fs;
    
    // Export NPCs as points
    let mut npc_obj = String::new();
    npc_obj.push_str("# NPCs\n");
    let mut _vid = 1;
    
    for npc in &map.entities.npcs {
        npc_obj.push_str(&format!("v {} {} {}\n", npc.pos_x as f32, npc.pos_y as f32, npc.pos_z as f32));
        npc_obj.push_str(&format!("o npc_{}\n", npc.type_id));
        _vid += 1;
    }
    
    if !map.entities.npcs.is_empty() {
        fs::write(output_dir.as_ref().join("npcs.obj"), npc_obj)?;
    }
    
    // Export warps
    let mut warp_obj = String::new();
    warp_obj.push_str("# Warps\n");
    for i in 0..10 {
        if map.entities.warps.target_map[i] >= 0 {
            warp_obj.push_str(&format!(
                "v {} {} {}\n",
                map.entities.warps.spawn_x[i] as f32,
                map.entities.warps.spawn_y[i] as f32,
                map.entities.warps.spawn_z[i] as f32
            ));
        }
    }
    if warp_obj.len() > 10 {
        fs::write(output_dir.as_ref().join("warps.obj"), warp_obj)?;
    }
    
    Ok(())
}

fn export_map_textures<P: AsRef<Path>>(_map: &MapFile, output_dir: P) -> Result<()> {
    use crate::formats::tim::TimHeader;
    use std::fs;
    use std::io::Cursor;
    
    let tex_dir = output_dir.as_ref().join("textures");
    fs::create_dir_all(&tex_dir)?;
    
    // We need the original file data to extract TIMs at specific offsets
    // This is a placeholder - in practice you'd pass the file data
    println!("Texture extraction requires original file data with TIM offsets");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_yaml_setup() {
        let setup = MapSetup {
            vpx: 100, vpy: 200, vpz: 300,
            vrx: 10, vry: 20, vrz: 30,
            lights: Default::default(),
            ambient_r: 128, ambient_g: 128, ambient_b: 128,
            projection: 1000,
            region_a: [1,2,3,4],
            region_b: [5,6,7,8],
            width: 32,
            height: 32,
            tiles: vec![0; 1024],
        };
        let yaml_setup: MapYamlSetup = (&setup).into();
        assert_eq!(yaml_setup.viewpoint, [100, 200, 300]);
        assert_eq!(yaml_setup.width, 32);
    }
}