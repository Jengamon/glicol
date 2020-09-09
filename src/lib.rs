extern crate pest;
#[macro_use]
extern crate pest_derive;

#[macro_use]
extern crate lazy_static;

use std::sync::{Mutex, Arc};
use std::{collections::HashMap, slice::from_raw_parts_mut};

use pest::Parser;
#[derive(Parser)]
#[grammar = "quaver.pest"]
pub struct QParser;

mod engine;
// use engine::{QuaverSignal, Event, QuaverLoop};
// use engine::instrument::{Sampler, QuaverFunction};
// instrument::Oscillator,
// use engine::effect::LPF;

// use dasp::{signal};
// use dasp::signal::Signal;
use engine::{SinOsc, Mul, Add, Impulse, Sampler, Looper};
use dasp_graph::{Buffer, Input, Node, NodeData, BoxedNode, BoxedNodeSend};
use petgraph::graph::{NodeIndex};

#[no_mangle] // to send buffer to JS
pub extern "C" fn alloc(size: usize) -> *mut f32 {
    let mut buf = Vec::<f32>::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr as *mut f32
}

#[no_mangle] // for receiving the String from JS
pub extern "C" fn alloc_uint8array(length: usize) -> *mut f32 {
    let mut arr = Vec::<u8>::with_capacity(length);
    let ptr = arr.as_mut_ptr();
    std::mem::forget(arr);
    ptr as *mut f32
}

#[no_mangle] // for receiving the String from JS
pub extern "C" fn alloc_uint32array(length: usize) -> *mut f32 {
    let mut arr = Vec::<u32>::with_capacity(length);
    let ptr = arr.as_mut_ptr();
    std::mem::forget(arr);
    ptr as *mut f32
}

lazy_static! {
    static ref ENGINE:Arc<Mutex<engine::Engine>> = Arc::new(Mutex::new(engine::Engine::new()));
    // static ref ENGINE:Mutex<engine::Engine> = Mutex::new(engine::Engine::new());
}

// Mutex<engine::Engine>
#[no_mangle]
pub extern "C" fn process(out_ptr: *mut f32, size: usize) {
    let mut engine = ENGINE.lock().unwrap();
    engine.process(out_ptr, size);
}

#[no_mangle]
pub extern "C" fn create_new_track(
    arr_ptr: *mut u8, length: usize,
    samples_ptr: *mut *mut f32, samples_len: usize,
    lengths_ptr: *mut *mut usize, lengths_len: usize,
    names_ptr: *mut *mut u8, names_len: usize,
    names_len_ptr: *mut *mut usize, names_len_len: usize
    ) {

    let mut samples_dict = HashMap::new();

    let mut engine = ENGINE.lock().unwrap();
    
    // an array containing all pointers of samples
    let samples: &mut [*mut f32] = unsafe {
        from_raw_parts_mut(samples_ptr, samples_len)};
    let lengths: &mut [*mut usize] = unsafe {
        from_raw_parts_mut(lengths_ptr, lengths_len)};
    let names: &mut [*mut u8] = unsafe {
        from_raw_parts_mut(names_ptr, names_len)};
    let names_len: &mut [*mut usize] = unsafe {
        from_raw_parts_mut(names_len_ptr, names_len_len)};
    
    // save samples in a HashMap
    for i in 0..samples.len() {
        let sample_array: &'static[f32] = unsafe {from_raw_parts_mut(samples[i], lengths[i] as usize)};
        // let st = unsafe {from_raw_parts_mut(samples[i], lengths[i] as usize)};
        // let sample_array = 
        let name_encoded:&mut [u8] = unsafe { from_raw_parts_mut(names[i], names_len[i] as usize) };
        let name = std::str::from_utf8(name_encoded).unwrap();
        samples_dict.insert(name, sample_array);
    };

    // read the code from the text editor
    let encoded:&mut [u8] = unsafe { from_raw_parts_mut(arr_ptr, length) };
    let quaver_code = std::str::from_utf8(encoded).unwrap();

    // parse the code
    let lines = QParser::parse(Rule::block, quaver_code)
    .expect("unsuccessful parse")
    .next().unwrap();

    // add function to Engine HashMap Function Chain Vec accordingly
    for line in lines.into_inner() {

        let mut ref_name = "~";
        // let mut func_chain = Vec::<Box<dyn Signal<Frame=f64> + 'static + Send>>::new(); // init Chain

        // match line.as_rule() {
        //     Rule::line => {
        let inner_rules = line.into_inner();
        // let mut func_vec = Vec::<Box<dyn QuaverFunction + 'static + Send>>::new();

        for element in inner_rules {
            match element.as_rule() {
                Rule::reference => {
                    ref_name = element.as_str();
                },
                Rule::chain => {
                    let mut node_vec = Vec::<NodeIndex>::new();
                    for func in element.into_inner() {
                        let mut inner_rules = func.into_inner();
                        let name: &str = inner_rules.next().unwrap().as_str();
                        match name {
                            "sin" => {
                                let mut paras = inner_rules.next().unwrap().into_inner();

                                // parsing 200 will cause error, 200.0 is fine.
                                let freq = paras.next().unwrap().as_str().parse::<f64>().unwrap();

                                let sin_osc = SinOsc::new(freq);
                                // let s_node = engine.graph.add_node(NodeData::new1(BoxedNode::new(Box::new(sin_osc))));
                                let sin_node = engine.graph.add_node(NodeData::new1(BoxedNodeSend::new(sin_osc)));
                                // engine.graph.add_node(NodeData::new1(BoxedNodeSend::new( Mul::new(0.5))));
                                
                                engine.nodes.insert(ref_name.to_string(), sin_node);
                                node_vec.insert(0, sin_node);

                                // let sig = SinOsc::new(freq);
                                // Add some nodes and edges...
                                // engine.graph.add_node(NodeData::new(sig, Vec::<Buffer>::new()));

                                // here we need to examine freq, if it is number, then make a consthz
                                // if it is ref, make a hz modulation
                                // let sig = signal::rate(48000.0).const_hz(freq).sine();
                                // func_chain.push(Box::new(SinOsc::new(freq)));

                            },
                            "mul" => {
                                let mut paras = inner_rules.next().unwrap().into_inner();
                                // let mul = paras.next().unwrap().as_str().parse::<f64>().unwrap();
                                // let mul = paras.next().unwrap().as_str().parse::<f64>();
                                let mul = paras.next().unwrap().as_str().to_string();
                                // if mul.is_ok() {

                                let mul_node = engine.graph.add_node(NodeData::new1(BoxedNodeSend::new( Mul::new(mul.clone()))));

                                if node_vec.len() > 0 {
                                    engine.graph.add_edge(node_vec[0], mul_node, ());
                                }
                                
                                engine.nodes.insert(ref_name.to_string(), mul_node);
                                node_vec.insert(0, mul_node);

                                let is_ref = !mul.parse::<f64>().is_ok();

                                if is_ref {
                                    if !engine.nodes.contains_key(mul.as_str()) {
                                        // panic if this item not existed
                                        // TODO: move it to a lazy function
                                        // engine.nodes.insert(mul.as_str().to_string(), mul_node);
                                    }                                    
                                    let mod_node = engine.nodes[mul.as_str()]; 
                                    engine.graph.add_edge(mod_node, mul_node, ());
                                }
                                // } else { // may be a ref

                                    // still need to add this
                                    // let mul_node = engine.graph.add_node(NodeData::new1(BoxedNodeSend::new(
                                    //     Mul::new(
                                    //         mul.unwrap()
                                    //     )
                                    // )));
                                // };

                                // match mul {
                                //     Ok(val) => {

                                //     },
                                //     Err(why) => {}
                                // }

                                // engine.node.push(mul_node);
                                // node_vec.push(mul_node);
                            },
                            "add" => {
                                let mut paras = inner_rules.next().unwrap().into_inner();
                                let add = paras.next().unwrap().as_str().parse::<f64>().unwrap();
                                let add_node = engine.graph.add_node(NodeData::new1(BoxedNodeSend::new( Add::new(add))));

                                if node_vec.len() > 0 {
                                    engine.graph.add_edge(node_vec[0], add_node, ());
                                }
                                
                                engine.nodes.insert(ref_name.to_string(), add_node);
                                node_vec.insert(0, add_node);
                                // engine.node.push(mul_node);
                                // node_vec.push(mul_node);
                            },
                            "loop" => {
                                // let mut q_loop = QuaverLoop::new();

                                let mut events = Vec::<(f64, f64)>::new();

                                let mut paras = inner_rules
                                .next().unwrap().into_inner();

                                let seq = paras.next().unwrap();
                                let mut compound_index = 0;
                                let seq_by_space: Vec<pest::iterators::Pair<Rule>> = 
                                seq.clone().into_inner().collect();

                                for compound in seq.into_inner() {
                                    let mut shift = 0;
            
                                    // calculate the length of seq
                                    let compound_vec: Vec<pest::iterators::Pair<Rule>> = 
                                    compound.clone().into_inner().collect(); 
            
                                    for note in compound.into_inner() {
                                        if note.as_str().parse::<i32>().is_ok() {
                                            let seq_shift = 1.0 / seq_by_space.len() as f64 * 
                                            compound_index as f64;
                                            
                                            let note_shift = 1.0 / compound_vec.len() as f64 *
                                            shift as f64 / seq_by_space.len() as f64;
            
                                            let d = note.as_str().parse::<i32>().unwrap() as f64;
                                            let relative_pitch = 2.0f64.powf((d - 60.0)/12.0);
                                            let relative_time = seq_shift + note_shift;
                                            events.push((relative_time, relative_pitch));
                                            // let mut event = Event::new();
                                           
                                            // event.pitch = pitch;

                                            // better to push a events, right?
                                            // q_loop.events.push(event);
                                        }
                                        shift += 1;
                                    }
                                    compound_index += 1;
                                }

                                let looper_node = engine.graph.add_node(
                                    NodeData::new1(BoxedNodeSend::new( Looper::new(events)))
                                );

                                if node_vec.len() > 0 {
                                    engine.graph.add_edge(node_vec[0], looper_node, ());
                                }
                                
                                engine.nodes.insert(ref_name.to_string(), looper_node);
                                node_vec.insert(0, looper_node);

                                // func_chain.functions.push(Box::new(q_loop));
                            },
                            "sampler" => {
                                let mut paras = inner_rules.next().unwrap().into_inner();
                                let symbol = paras.next().unwrap().as_str();

                                let sampler_node = engine.graph.add_node(
                                    NodeData::new1(BoxedNodeSend::new( Sampler::new(samples_dict[symbol])))
                                );

                                if node_vec.len() > 0 {
                                    engine.graph.add_edge(node_vec[0], sampler_node, ());
                                }
                                
                                engine.nodes.insert(ref_name.to_string(), sampler_node);
                                node_vec.insert(0, sampler_node);
                                // sig.ins.push(
                                //     Box::new(
                                //         Sampler::new(samples_dict[symbol].clone())
                                //     )
                                // );
                                // func_chain.functions.push(
                                //     Box::new(Sampler::new(samples_dict[symbol].clone()))
                                // );
                            },
                            "imp" => {
                                let mut paras = inner_rules.next().unwrap().into_inner();
                                let imp = paras.next().unwrap().as_str().parse::<f64>().unwrap();
                                let imp_node = engine.graph.add_node(
                                    NodeData::new1(BoxedNodeSend::new( Impulse::new(imp)))
                                );

                                if node_vec.len() > 0 {
                                    engine.graph.add_edge(node_vec[0], imp_node, ());
                                }
                                
                                engine.nodes.insert(ref_name.to_string(), imp_node);
                                node_vec.insert(0, imp_node);
                            },
                            "lpf" => {
                            },
                            "lfo" => {
                            },
                            _ => unreachable!()
                        }
                        // create the edge here
                        // if node_vec.len() == 2 {
                        //     engine.graph.add_edge(node_vec[0], node_vec[1], ());
                        //     node_vec.clear();
                        // }
                       

                    }
                },
                _ => unreachable!()
            }
        }
        // engine.chains.insert(ref_name.to_string(), func_chain); // sig: sig_chain
    };
    // engine.phase = 0;
}