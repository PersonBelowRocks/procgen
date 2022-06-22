# Minecraft procedural terrain generation server.
This is an attempt at making a terrain generation server for Minecraft in Rust.
The system (hopefully) works like this:
- A terrain server is started and equipped with some terrain generators.
- Some Minecraft server with a terrain client (in the form of a plugin) connects to the terrain server when it feels like it.
- The client then (should) request to create a generator instance on the server, the client will specify various parameters for the instance in the packet.
- This will make the terrain server invoke the generator's factory, creating a new instance with the parameters provided by the client.
- The server then replies to the client with a "generator ID", a unique ID for the instance it just created.
- The client is responsible for keeping track of generator IDs for the generator instances that it has created.
- After an instance has been created the client may send a "generation request" packet to the server, specifying the location of the chunk it wants generated
and the generator ID of the instance it wants to generate the chunk with.
- The server then uses the instance to generate the chunk, and sends it back to the client.

When a client requests to create a generator instance or requests to generate a chunk, it must provide a "request ID" for the request.
Later, when the server replies, it'll attach this ID to identify what request the packet is related to.
The client is responsible for keeping track of which request IDs refer to which requests.

The system should (fingers crossed) allow clients to use the server to generate all sorts of different terrain, and use multiple generators.
One scenario where this would be useful is having different generators for different dimensions (one nether, one end, one overworld).

### Progress and plans
Currently the server is "usable", but there is no client or plugin for it. 
The server is also riddled with various bugs, for example:
- A client can access a generator instance of another client if they know the instance's ID.
- A client can trigger a panic in the server by doing various simple things (like a malformed packet).
- There's no way to really terminate a running server at the moment, so the only choice is to crash.

There's many more bugs on top of this, most of which are just there because I'm very lazy and just want to see it work first.

Plans for the future include:
- A standalone crate for the networking functionality (mainly just for packets).
- A client/plugin to use on a Minecraft server.
- An API for writing generators more easily, lots of fun to be had here!
- An API or maybe even an own component for structure generation (villages, strongholds, etc.). It'd be cool to import
schematica files for this, so you'd build structures in-game and just import them and the server would do the rest.
- (big maybe for this one!) A system for generating all sorts of stuff, and at any time, not just chunks. For example,
the server could generate trees for you and you had a plugin to just grab procedural trees from the server and place them in the world.
Very nice for building and terraforming!
- General improvements and cleanup. Everything from code quality to performance to documentation!

