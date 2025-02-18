use csv::Reader;
use nalgebra::geometry::{Point3, Rotation3};
use nannou::prelude::*;
use rand::Rng;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

const WINDOW_SIZE: u32 = 1200;
const SCALE: f32 = 0.25;
const SPHERE_SIZE: f32 = WINDOW_SIZE as f32 * SCALE;
const N_BORBS: usize = 150;
const BORB_SPEED: f32 = 0.01;
const BREAK_COUNT: usize = 5;
const MOUSE_SENSITIVITY: f32 = 0.01;

struct Node {
    pos: Point3<f32>,
}

impl Node {
    fn fade(&self) -> f32 {
        0.5 + (self.pos.z - WINDOW_SIZE as f32 / 2.0) / (WINDOW_SIZE as f32)
    }
}

#[derive(Debug, Deserialize)]
struct Edge {
    src: usize,
    dest: usize,
    hop_count: usize,
    free: bool,
}

struct Borb {
    pos: Point3<f32>,
    dest_pos: Point3<f32>,
    src: usize,
    dest: usize,
    progress: f32,
    color: (f32, f32, f32),
}

impl Borb {
    fn spawn_random(nodes: &[Node], neighbors: &[Vec<usize>]) -> Self {
        let src = rand::thread_rng().gen_range(0..nodes.len());
        let options = &neighbors[src];
        let index: usize = rand::thread_rng().gen_range(0..options.len());
        let dest = options[index];

        Self {
            pos: nodes[src].pos,
            dest_pos: nodes[dest].pos,
            src,
            dest,
            progress: 0.0,
            color: (1.0, 0.1, 0.1),
        }
    }

    fn hop(&mut self, nodes: &[Node], options: &[Vec<usize>]) {
        if !options[self.dest].is_empty() {
            self.src = self.dest;
            self.pos = self.dest_pos;
            let index = rand::thread_rng().gen_range(0..options[self.dest].len());
            let dest = options[self.dest][index];
            self.dest = dest;
            self.dest_pos = nodes[dest].pos;
            self.progress = 0.0;
        }
    }

    fn step(&mut self) {
        self.progress += BORB_SPEED;
        self.pos = self.pos + self.progress * (self.dest_pos - self.pos);
    }

    fn fade(&self) -> f32 {
        0.5 + (self.pos.z - SPHERE_SIZE / 2.0) / (SPHERE_SIZE)
    }

    fn size(&self) -> f32 {
        2.0 + 25.0 * (self.progress - 0.25).abs()
    }
}

fn main() {
    nannou::app(model).update(update).run();
}

struct Model {
    nodes: Vec<Node>,
    edges: HashMap<(usize, usize), Edge>,
    borbs: Vec<Borb>,
    neighbors: Vec<Vec<usize>>,
    delta_angles: (f32, f32),
    mouse_dragging: bool,
    last_mouse_position: Point2,
}

impl Model {
    fn new(nodes: Vec<Node>, edges: HashMap<(usize, usize), Edge>) -> Self {
        let mut neighbors: Vec<Vec<usize>> = vec![Vec::<usize>::new(); nodes.len()];
        for ((src, dest), _) in edges.iter() {
            neighbors[*src].push(*dest);
            neighbors[*dest].push(*src);
        }

        let mut borbs: Vec<Borb> = Vec::new();
        for _ in 0..N_BORBS {
            borbs.push(Borb::spawn_random(&nodes, &neighbors));
        }

        Model {
            nodes,
            edges,
            borbs,
            neighbors,
            delta_angles: (0.0, 0.0),
            mouse_dragging: false,
            last_mouse_position: Point2::new(0.0, 0.0),
        }
    }
}

fn model(app: &App) -> Model {
    let _window_id = app
        .new_window()
        .size(WINDOW_SIZE, WINDOW_SIZE)
        .view(view)
        .mouse_moved(mouse_moved)
        .mouse_pressed(mouse_pressed)
        .mouse_released(mouse_released)
        .build()
        .unwrap();

    let pos_f = PathBuf::from("graphs/50_node/graph_positions.csv");
    let edge_f = PathBuf::from("graphs/50_node/graph_edges.csv");
    let (nodes, edges) = read_graph(&pos_f, &edge_f);

    Model::new(nodes, edges)
}

fn update(_app: &App, model: &mut Model, _update: Update) {
    let Model {
        ref mut nodes,
        ref mut edges,
        ref mut borbs,
        ref mut neighbors,
        ..
    } = *model;

    // Rotating points

    let mut broken_edges: Vec<(usize, usize)> = Vec::with_capacity(edges.len());
    for (key, edge) in edges.iter() {
        if edge.hop_count >= BREAK_COUNT {
            broken_edges.push(*key);
        }
    }

    let num_nodes = nodes.len();
    for (src, dest) in broken_edges {
        if let Some(pos) = neighbors[src].iter().position(|x| *x == dest) {
            neighbors[src].remove(pos);
        }
        if let Some(pos) = neighbors[dest].iter().position(|x| *x == src) {
            neighbors[dest].remove(pos);
        }

        let new_dest = rand::thread_rng().gen_range(0..num_nodes);
        let new_edge: Edge = Edge {
            src,
            dest: new_dest,
            hop_count: 0,
            free: true,
        };
        edges.insert((src, new_dest), new_edge);
        neighbors[src].push(new_dest);
        let new_dest = rand::thread_rng().gen_range(0..num_nodes);
        let new_edge: Edge = Edge {
            src: dest,
            dest: new_dest,
            hop_count: 0,
            free: true,
        };
        edges.insert((dest, new_dest), new_edge);
        neighbors[dest].push(new_dest);
        edges.remove(&(src, dest));
    }

    if model.mouse_dragging {
        let r = Rotation3::from_euler_angles(model.delta_angles.0, model.delta_angles.1, 0.0);
        for n in model.nodes.iter_mut() {
            n.pos = r * n.pos;
        }
        for b in borbs.iter_mut() {
            b.pos = r * b.pos;
            b.dest_pos = r * b.dest_pos;
        }
    } else {
        let r: Rotation3<f32> = Rotation3::from_euler_angles(0.0, 0.005, 0.0);

        for n in nodes.iter_mut() {
            n.pos = r * n.pos;
        }
        for b in borbs.iter_mut() {
            b.pos = r * b.pos;
            b.dest_pos = r * b.dest_pos;
        }
    }

    // Step Objects
    for borb in borbs.iter_mut() {
        borb.step();
        if borb.progress >= 0.5 {
            borb.hop(&model.nodes, &model.neighbors);
            if let Some(e) = edges.get_mut(&(borb.src, borb.dest)) {
                e.hop_count += 1;
            }
            if let Some(e) = edges.get_mut(&(borb.dest, borb.src)) {
                e.hop_count += 1;
            }
        }
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    draw.background().rgba(0.0, 0.0, 0.0, 0.75);
    draw_model(&draw, model);
    draw.to_frame(app, &frame).unwrap();
}

fn draw_model(draw: &Draw, model: &Model) {
    // Drawing edges
    for ((src, dest), e) in model.edges.iter() {
        let n1 = &model.nodes[*src];
        let n2 = &model.nodes[*dest];

        let fade: f32 = (n1.fade() + n2.fade()) / 2.0;
        let rc = e.hop_count as f32 / BREAK_COUNT as f32;
        if !e.free {
            draw.line()
                .start(vec2(n1.pos.x, n1.pos.y))
                .end(vec2(n2.pos.x, n2.pos.y))
                .weight(7.0)
                .rgba(rc, 0.75 - rc, 0.75 - rc, fade);
        }
    }

    for borb in model.borbs.iter() {
        draw.ellipse()
            .x_y_z(borb.pos.x, borb.pos.y, borb.pos.z)
            .radius(borb.size())
            .rgba(borb.color.0, borb.color.1, borb.color.2, borb.fade());
    }
}

#[derive(Debug, Deserialize)]
struct NodeReader {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Debug, Deserialize)]
struct EdgeReader {
    src: usize,
    dest: usize,
}

fn read_graph(
    pos_file: &PathBuf,
    edge_file: &PathBuf,
) -> (Vec<Node>, HashMap<(usize, usize), Edge>) {
    let mut nodes: Vec<Node> = Vec::new();
    let mut edges: HashMap<(usize, usize), Edge> = HashMap::new();
    let mut rdr = Reader::from_path(pos_file).unwrap();
    for result in rdr.deserialize() {
        let n: NodeReader = result.unwrap();
        let node: Node = Node {
            pos: SPHERE_SIZE * Point3::new(n.x, n.y, n.z),
        };
        nodes.push(node);
    }
    let mut rdr = Reader::from_path(edge_file).unwrap();
    for result in rdr.deserialize() {
        let e: EdgeReader = result.unwrap();
        let edge: Edge = Edge {
            src: e.src,
            dest: e.dest,
            hop_count: 0,
            free: false,
        };
        edges.insert((edge.src, edge.dest), edge);
    }

    (nodes, edges)
}

fn mouse_moved(_app: &App, model: &mut Model, position: Point2) {
    if model.mouse_dragging {
        let delta_x = (position.x - model.last_mouse_position.x) * MOUSE_SENSITIVITY;
        let delta_y = -(position.y - model.last_mouse_position.y) * MOUSE_SENSITIVITY;
        model.delta_angles = (delta_y, delta_x);
        model.last_mouse_position = position;
    } else {
        model.last_mouse_position = position;
    }
}

fn mouse_pressed(_app: &App, model: &mut Model, _button: MouseButton) {
    model.mouse_dragging = true;
}

fn mouse_released(_app: &App, model: &mut Model, _button: MouseButton) {
    model.mouse_dragging = false;
}
