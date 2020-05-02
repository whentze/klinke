use klinke::{base_modules::*, engine::AudioGraph};

fn main() {
    let mut graph = AudioGraph::start();

    let vco = graph.add_module(Vco::default());
    let sine = graph.add_module(Sine::default());

    graph.connect(sine, 0, vco, 0);

    graph.designate_output(vco, 0);

    loop{};
}
