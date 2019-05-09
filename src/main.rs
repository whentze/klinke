use klinke::{base_modules::*, engine::AudioGraph};

fn main() {
    let mut graph = AudioGraph::start();

    let sine = graph.add_module(Sine::new());

    graph.designate_output(sine, 0);

    loop{};
}
