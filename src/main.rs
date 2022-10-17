use libp2p::{ core::upgrade
            , floodsub::{ Floodsub, FloodsubEvent, Topic                             }
            , futures::StreamExt
            , identity
            , mdns::{ Mdns, MdnsEvent                                                }
            , mplex
            , noise::{ Keypair, NoiseConfig, X25519Spec                              }
            , swarm::{ NetworkBehaviourEventProcess, Swarm, SwarmBuilder, SwarmEvent }
            , tcp::TokioTcpConfig
            , Multiaddr 
            , NetworkBehaviour
            , PeerId
            , Transport
}; //use libp2p::{ core::upgrade

use once_cell::sync::Lazy                     ;
use serde::{ Deserialize, Serialize          };
use sha2::{ Digest, Sha256                   };
use std::{ thread, time                      };
use tokio::{ io::AsyncBufReadExt, sync::mpsc };

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Block { previous: String
             , text    : String
             }
#[derive(Debug, Deserialize, Serialize)]
struct Letter { chain   : Vec<Block>
              , receiver: String
              , request : bool
              }
#[derive(Debug, Deserialize, Serialize)]
struct Request { destination: String } 

#[derive(Debug, Deserialize, Serialize)]
struct Response { chain   : Vec<Block>
                , receiver: String
                }
#[derive(NetworkBehaviour)]
struct Behaviour { #[behaviour(ignore)] chain   : Vec<Block>
                 ,                      floodsub: Floodsub
                 ,                      mdns    : Mdns    
                 , #[behaviour(ignore)] peer    : PeerId
                 , #[behaviour(ignore)] sender  : mpsc::UnboundedSender<Letter>
                 }
enum Event { Address(Multiaddr)
           , Dialing(PeerId)
           , Input(String)
           , Letter(Letter)
           }
pub static DELAY: Lazy<u64> = Lazy::new(|| 400);

fn block(chain: &Vec<Block>, text: &String) -> Block {
 let mut previous = "                                                                ".to_string();

 if chain.len() > 0 {
  previous = hash(&chain.last().unwrap());

 }//if chain.len() > 0 {

 let block = Block { previous, text: text.clone() };

 block
}//fn block(chain: &Vec<Block>, text: &String) -> Block {

fn hash(block: &Block) -> String {
 let mut hasher = Sha256::new();

 hasher.update(serde_json::json!({"previous": block.previous, "text": block.text }).to_string().as_bytes());

 hex::encode(hasher.finalize().as_slice().to_owned())
}//fn hash(block: &Block) -> String {

fn print(chain: &Vec<Block>) {
 for link in chain { 
  println!("{:?}", link); thread::sleep(time::Duration::from_millis(*DELAY));

 }//for link in chain { 
}//fn print(chain: &Vec<Block>) {

fn push(block: &Block, chain: &mut Vec<Block>) {
 if chain.len() > 0 {
  let last = chain.last().expect("there is at last block");

  if hash(last) == block.previous {
   chain.push(block.clone());

  }//if hash(last) == block.previous {

 } else {//if chain.len() > 0 {
  chain.push(block.clone());

 }//} else {//if chain.len() > 0 {

 print(&chain);
}//fn push(block: &Block, chain: &mut Vec<Block>) {

impl NetworkBehaviourEventProcess<FloodsubEvent> for Behaviour {
 fn inject_event(&mut self, event: FloodsubEvent) {
  match event {
   FloodsubEvent::Message(message) => {
    println!("message.source: {:?}", message.source); thread::sleep(time::Duration::from_millis(*DELAY));

    if let Ok(block) = serde_json::from_slice::<Block>(&message.data) {
     println!("{:?}", block); thread::sleep(time::Duration::from_millis(*DELAY));

     push(&block, &mut self.chain);

    } else if let Ok(letter) = serde_json::from_slice::<Letter>(&message.data) {
     println!("{:?}", letter); thread::sleep(time::Duration::from_millis(*DELAY));

     if letter.request {
      if letter.receiver.trim().is_empty() || letter.receiver == self.peer.to_string() {
       if let Err(e) = self.sender.clone().send( Letter { chain: self.chain.clone(), receiver: message.source.to_string(), request: false } ) { 
        println!("{:?}", e); thread::sleep(time::Duration::from_millis(*DELAY));

       }//if let Err(e) = self.sender.clone().send( Letter { chain: self.chain.clone(), receiver: message.source.to_string(), request: false } ) { 
      }//if letter.receiver.trim().is_empty() || letter.receiver == self.peer.to_string() {

     } else {//if letter.request {
      if letter.receiver == self.peer.to_string() {
       if letter.chain.len() > self.chain.len() {
        self.chain = letter.chain;

        print(&self.chain);
       }//if letter.chain.len() > self.chain.len() {
      }//if letter.receiver == self.peer.to_string() {
     }//} else {//if letter.request {
    }//} else if let Ok(letter) = serde_json::from_slice::<Letter>(&message.data) {
   }//FloodsubEvent::Message(message) => {

   _ => ()
  }//match event {
 }//fn inject_event(&mut self, event: FloodsubEvent) {
}//impl NetworkBehaviourEventProcess<FloodsubEvent> for Behaviour {

impl NetworkBehaviourEventProcess<MdnsEvent> for Behaviour {
 fn inject_event(&mut self, event: MdnsEvent) {
  match event {
   MdnsEvent::Discovered(discovered) => { 
    for (peer, addr) in discovered {         
     self.floodsub.add_node_to_partial_view(peer);   

     println!("discovered: {:?}, {:?}", peer, addr); thread::sleep(time::Duration::from_millis(*DELAY)); 
    }//for (peer, addr) in discovered {         
   }//MdnsEvent::Discovered(discovered) => { 

   MdnsEvent::Expired(expired) => { 
    for (peer, addr) in expired {
     if !self.mdns.has_node(&peer) { 
      self.floodsub.remove_node_from_partial_view(&peer);

      println!("discovered: {:?}, {:?}", peer, addr); thread::sleep(time::Duration::from_millis(*DELAY)); 
     }//if !self.mdns.has_node(&peer) { 
    }//for (peer, addr) in expired {
   }//MdnsEvent::Expired(expired) => { 
  }//match event {
 }//fn inject_event(&mut self, event: MdnsEvent) {
}//impl NetworkBehaviourEventProcess<MdnsEvent> for Behaviour {

#[tokio::main]
async fn main() {
 let identity: identity::Keypair = identity::Keypair::generate_ed25519();

 let peer: PeerId = PeerId::from(identity.public());

 let (sender, mut recipient) = mpsc::unbounded_channel();

 let auth_keys = Keypair::<X25519Spec>::new().into_authentic(&identity).expect("can create auth keys");

 let transp = TokioTcpConfig::new().upgrade(upgrade::Version::V1).authenticate(NoiseConfig::xx(auth_keys).into_authenticated()).multiplex(mplex::MplexConfig::new()).boxed();

 let topic: Topic = Topic::new("text");

 let mut behaviour = Behaviour { chain: vec![], floodsub: Floodsub::new(peer.clone()), mdns: Mdns::new(Default::default()).await.expect("can create mdns"), peer, sender };

 behaviour.floodsub.subscribe(topic.clone());

 let mut swarm = SwarmBuilder::new(transp, behaviour, peer.clone()).executor(Box::new(|fut| { tokio::spawn(fut); })).build();

 let mut stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();

 Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/0".parse().expect("can get a local socket")).expect("swarm can be started");

 println!("peer.to_string(): {:?}", peer.to_string()); thread::sleep(time::Duration::from_millis(*DELAY));

 loop { 
  let option = { 
   tokio::select! { event  = swarm.select_next_some() => { match event { SwarmEvent::NewListenAddr { address, .. } => { Some(Event::Address(address)) } 
                                                                         SwarmEvent::Dialing(peer_id)              => { Some(Event::Dialing(peer_id)) }
                                                                         _                                         =>   None                          
                                                                       } 
                                                         }
                  , line   = stdin.next_line()        => Some(Event::Input(line.expect("can get line").expect("can read line from stdin")))
                  , letter = recipient.recv()         => Some(Event::Letter(letter.expect("letter exists")))
                  }
  };//let option = { 

  if let Some(event) = option {
   match event {
    Event::Address(address) => {
     println!("Event::Address(address): {:?}", address); thread::sleep(time::Duration::from_millis(*DELAY));

    }//Event::Address(address) => {

    Event::Dialing(peer_id) => {
     println!("Event::Dialing(peer_id):       {:?}", peer_id                      ); thread::sleep(time::Duration::from_millis(*DELAY));
     println!("swarm.behaviour().chain.len(): {:?}", swarm.behaviour().chain.len()); thread::sleep(time::Duration::from_millis(*DELAY));

     if swarm.behaviour().chain.len() == 0 {
      if let Err(e) = swarm.behaviour_mut().sender.clone().send(Letter { chain: vec![], receiver: peer_id.to_string(), request: true }) {
       println!("{:?}", e); thread::sleep(time::Duration::from_millis(*DELAY));

      }//if let Err(e) = swarm.behaviour_mut().sender.clone().send(Letter { chain: vec![], receiver: peer_id.to_string(), request: true }) {
     }//if swarm.behaviour().chain.len() == 0 {
    }//Event::Dialing(peer_id) => {

    Event::Input(line) => {
     println!("Event::Input(line): {:?}", line); thread::sleep(time::Duration::from_millis(*DELAY));

     match &line[..] { "exit" => break
                     , "size" => println!("swarm.behaviour().mdns.discovered_nodes().len(): {:?}", swarm.behaviour().mdns.discovered_nodes().len())
                     , _      => if line.len() == 52 { 
                                  if swarm.behaviour().mdns.discovered_nodes().find(|&&x| x.to_string() == line).expect("covert PeerId to String").to_string() == line {
                                   let letter = Letter { chain: vec![], receiver: line, request: true }                                                         ;
                                   swarm.behaviour_mut().floodsub.publish(topic.clone(), serde_json::to_string(&letter).expect("can jsonify letter").as_bytes());
                                   println!("{:?}", letter); thread::sleep(time::Duration::from_millis(*DELAY))                                                 ;
                                  }//if swarm.behaviour().mdns.discovered_nodes().find(|&&x| x.to_string() == line).to_string() == line {

                                 } else {//if line.len() == 52 { 
                                  let block = block(&swarm.behaviour_mut().chain, &line)                                                                       ;
                                  push(&block, &mut swarm.behaviour_mut().chain)                                                                               ;
                                  swarm.behaviour_mut().floodsub.publish(topic.clone(), serde_json::to_string(&block).expect("can jsonify request").as_bytes());
                                 }//} else {//if line.len() == 52 {
     }//match &line[..] {
    }//Event::Input(line) => {

    Event::Letter(letter) => {
     swarm.behaviour_mut().floodsub.publish(topic.clone(), serde_json::to_string(&letter).expect("can jsonify letter").as_bytes());
     println!("Event::Letter(letter): {:?}", letter); thread::sleep(time::Duration::from_millis(*DELAY))                          ;
    }//Event::Letter(letter) => {
   }//match event {
  }//if let Some(event) = option {
 }//loop {
}//async fn main() {

