use crossbeam_channel::{Sender, Receiver, bounded};
use generational_arena::{Arena, Index};
use petgraph::graphmap::DiGraphMap;
use portaudio as pa;

use std::thread;

use crate::{alloc::become_audio_thread, cables::{Input, Output}, config::{BUFFER_SIZE, SAMPLE_RATE}, Module};

#[derive(Debug)]
struct Cable {
    from: u8,
    to: u8,
}

#[derive(Debug)]
pub enum EngineCommand {
    AddNode(Node),
    RemoveNode(Index),
    Connect{
        out_node: Index,
        out_port: u8,
        in_node: Index,
        in_port: u8,
    },
    Disconnect {
        node: Index,
        port: u8,
    },
    DesignateOutput {
        node: Index,
        port: u8,
    },
    NewSchedule(Box<[Index]>),
}

#[derive(Debug)]
pub enum EngineResponse {
    NodeAdded(Index),
    NodeRemoved(Option<Node>),
}

#[derive(Debug)]
pub struct AudioGraph {
    graph: DiGraphMap<Index, Cable>,

    commands: crossbeam_channel::Sender<EngineCommand>,
    responses: crossbeam_channel::Receiver<EngineResponse>,
}

impl AudioGraph {
    pub fn start() -> Self {
        let (c_tx, c_rx) = bounded(128);
        let (r_tx, r_rx) = bounded(128);
        thread::spawn(move || {
            let mut engine = AudioEngine::with_capacity(1024, c_rx, r_tx);
            
            let pa = pa::PortAudio::new().unwrap();
            
            let mut settings = pa.default_output_stream_settings(1, SAMPLE_RATE as f64, BUFFER_SIZE as u32).unwrap();
            settings.flags = pa::stream_flags::CLIP_OFF;

            let callback = move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
                for idx in 0..frames {
                    engine.run();
                    buffer[idx] = engine.audio_out.get();
                }
                pa::Continue
            };

            let mut stream = pa.open_non_blocking_stream(settings, callback).unwrap();

            become_audio_thread();
            stream.start().unwrap();

            loop{}
        });

        Self {
            graph: DiGraphMap::new(),

            commands: c_tx,
            responses: r_rx,
        }
    }
    pub fn add_module<M: Module>(&mut self, module: M) -> Index {
        let inputs = (0..module.num_inputs()).map(|_| Input::default()).collect::<Vec<_>>().into_boxed_slice();
        let outputs = (0..module.num_outputs()).map(|_| Output::default()).collect::<Vec<_>>().into_boxed_slice();

        self.commands.send(EngineCommand::AddNode(Node {
            inputs,
            outputs,

            module: Box::new(module),
        })).unwrap();

        let i = match self.responses.recv().unwrap() {
            EngineResponse::NodeAdded(i) => i,
            _ => panic!("audio thread responded with an unexpected message!"),
        };
        self.graph.add_node(i);
        self.recompute_schedule();

        i
    }
    pub fn remove_module(&mut self, index: Index) -> Option<Box<Module>> {
        self.commands.send(EngineCommand::RemoveNode(index)).unwrap();

        match self.responses.recv().unwrap() {
            EngineResponse::NodeRemoved(o) => o.map(|n| n.module),
            _ => panic!("audio thread responded with an unexpected message!"),
        }
    }

    pub fn connect(&mut self, out_node: Index, out_port: u8, in_node: Index, in_port: u8) {
        self.commands.send(EngineCommand::Connect{out_node, out_port, in_node, in_port}).unwrap();
    }
    pub fn disconnect(&mut self, node: Index, port: u8) {
        self.commands.send(EngineCommand::Disconnect{node, port}).unwrap();
    }

    pub fn designate_output(&mut self, node: Index, port: u8) {
        self.commands.send(EngineCommand::DesignateOutput{node, port}).unwrap();
    }
    fn recompute_schedule(&mut self) {
        let sched = self.graph.nodes().collect::<Vec<_>>().into_boxed_slice();
        self.commands.send(EngineCommand::NewSchedule(sched)).unwrap();
    }
}




#[derive(Debug)]
pub struct Node {
    pub(crate) inputs: Box<[Input]>,
    pub(crate) outputs: Box<[Output]>,

    pub(crate) module: Box<dyn Module>,
}

impl Node {
    fn run(&mut self) {
        self.module.run(&*self.inputs, &*self.outputs)
    }
}

pub struct AudioEngine {
    nodes: Arena<Node>,
    schedule: Box<[Index]>,
    audio_out: Input,

    commands: crossbeam_channel::Receiver<EngineCommand>,
    responses: Sender<EngineResponse>,
}

impl AudioEngine {
    pub fn with_capacity(cap: usize, commands: Receiver<EngineCommand>, responses: Sender<EngineResponse>) -> Self {
        Self {
            nodes: Arena::with_capacity(cap),
            schedule: Box::from([]),
            audio_out: Input::default(),

            commands, responses,
        }
    }

    pub fn run(&mut self) {
        // process audio
        for i in &*self.schedule {
            let node = self.nodes.get_mut(*i).unwrap();
            node.run();
        }
        // maybe process a command
        use EngineCommand::*;
        use EngineResponse::*;
        match self.commands.try_recv() {
            Ok(AddNode(n)) => {
                let idx = self.insert_node(n);
                self.responses.try_send(NodeAdded(idx)).unwrap();
            },
            Ok(RemoveNode(i)) => {
                let old_node = self.remove_node(i);
                self.responses.try_send(NodeRemoved(old_node)).unwrap();
            },
            Ok(Connect{out_node, out_port, in_node, in_port}) => {
                self.connect(out_node, out_port, in_node, in_port);
            },
            Ok(Disconnect{node, port}) => {
                self.disconnect(node, port);
            },
            Ok(NewSchedule(s)) => {
                self.schedule = s;
            }
            Ok(DesignateOutput{node, port}) => {
                self.designate_output(node, port);
            },
            Err(_) => {},
        }
    }

    fn insert_node(&mut self, node: Node) -> Index {
        self.nodes.try_insert(node).unwrap()
    }

    fn remove_node(&mut self, i: Index) -> Option<Node> {
        self.nodes.remove(i).map(|old_node| {
            for (_, node) in self.nodes.iter_mut() {
                for input in node.inputs.iter_mut() {
                    if input.points_within(&*old_node.outputs) {
                        input.disconnect();
                    }
                }
            };
            old_node
        })
    }

    fn connect(&mut self, out_node: Index, out_port: u8, in_node: Index, in_port: u8) {
        if in_node == out_node {
            // self-loop
            let node = self.nodes.get_mut(in_node).unwrap();
            unsafe { node.inputs[in_port as usize].connect_to(&mut node.outputs[out_port as usize]) };
        } else {
            match self.nodes.get2_mut(out_node, in_node) {
                (Some(o), Some(i)) => unsafe { i.inputs[in_port as usize].connect_to(&mut o.outputs[out_port as usize]) },
                _ => panic!("trying to connect nonexisting node(s)."),
            }
        }
    }

    fn disconnect(&mut self, node: Index, port: u8) {
        let node = self.nodes.get_mut(node).unwrap();
        node.inputs[port as usize].disconnect();
    }

    fn designate_output(&mut self, node: Index, port: u8) {
        let node = self.nodes.get_mut(node).unwrap();
        unsafe { self.audio_out.connect_to(&mut node.outputs[port as usize]) };
    }
}