use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
use std::fmt::Display;
use std::sync::Arc;
use once_cell::sync::Lazy;
use serenity::http::AttachmentType;
use serenity::model::id::GuildId;
use serenity::prelude::RwLock;
use serenity::utils::Color;

use serde_json::{self, Value};

use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::model::channel::Message;
use serenity::framework::standard::{
    StandardFramework,
    CommandResult,
    macros::{
        command,
        group
    }
};
use tokio::net::TcpStream;

#[group]
#[commands(ping, help, add, remove, setactive, status, statusip, removeall, servers)]

struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {

    let prefix = env::var("DISCORD_PREFIX").expect("prefix");

    let framework = StandardFramework::new()
        .configure(|c| c.prefix(&prefix)) // set the bot's prefix to "~"
        .group(&GENERAL_GROUP);

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("token");
    let mut client = Client::builder(token)
    // let mut client = Client::builder("OTE5NTA0NTIwNDAzMzY5OTk0.YbWxUQ.pmgDq8xxPX74OZnW3uc4UWDq9dY")
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {}", why);
    }
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "Pong!").await?;

    Ok(())
}


mod network;

#[derive(Debug, Clone)]
struct MCServer {
    pub ip: String,
    pub name: Option<String>,
}

impl MCServer {
    pub fn new(ip: String, name: Option<String>) -> MCServer {

        let mut ip = ip;

        let mut stuff = ip.split(":");
        stuff.next();

        let port = stuff.next();

        if port.is_none() {
            ip.push_str(":25565");
        }

        MCServer {
            ip,
            name,
        }
    }
}

#[derive(Debug, Clone)]
struct GuildServers {
    pub active: usize,
    pub servers: Vec<MCServer>,
}

impl Display for MCServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.name {
            Some(nam) => {
                write!(f, "{} ({})", nam, self.ip)
            },
            None => {
                write!(f, "{}", self.ip)
            }
        }
    }
}

static SERVERS: Lazy<Arc<RwLock<HashMap<GuildId, GuildServers>>>> = Lazy::new(|| Arc::new(RwLock::new(HashMap::new()))); 

async fn check_guild_server_exists(id: &GuildId) {

    println!("Checking guild server exists");

    {
        let map = SERVERS.read().await;

        if map.contains_key(id) {
            return;
        }
    }

    let mut map = SERVERS.write().await;

    map.insert(*id, GuildServers {
        active: 0,
        servers: Vec::new(),
    });
}

#[command]
async fn help(ctx: &Context, msg: &Message) -> CommandResult {

    msg.channel_id.send_message(&ctx.http, |m| {
        m.embed(|e| {

            e.title("Commands");

            e.field("help", "Open this menu", false);
            e.field("add <ServerName> <ServerIP>", "Adds a Minecraft server with a name to the list", false);
            e.field("remove <ServerName>", "Removes a server from the list", false);
            e.field("removeall", "Removes all servers from the list", false);
            e.field("setactive <ServerName>", "Sets a server as the active one so that running `status` automatically uses that one", false);
            e.field("servers", "Lists all server currently in the list", false);
            e.field("status", "Gets the status of the Minecraft server currently set as active", false);
            e.field("status <ServerName>", "Gets the status of the saved Minecraft server with that name", false);
            e.field("statusip <ServerIP>", "Gets the status of the Minecraft server at that IP, it does not need to be saved for this to work", false);

        e});
        m}).await?;


    Ok(())
}

#[command]
async fn add(ctx: &Context, msg: &Message) -> CommandResult {

    let mut stuff = msg.content.split(" ");
    stuff.next();

    let name = stuff.next();
    let ip = stuff.next();

    if name.is_none() || ip.is_none() {
        msg.reply(ctx, "Improper command uages. Proper use:\nadd <ServerName> <ServerIP>").await?;
        return Ok(());
    }

    let name = name.unwrap().to_string();
    let ip = ip.unwrap().to_string();

    let id = &msg.guild_id.unwrap();

    let mut servs = SERVERS.write().await;

    match servs.get_mut(id) {
        Some(gs) => {
            gs.servers.push(MCServer::new(ip.clone(), Some(name.clone())));
        },
        None => {
            let gs = GuildServers {
                active: 0,
                servers: vec![MCServer::new(ip.clone(), Some(name.clone()))],
            };
            servs.insert(*id, gs);
        }
    }

    msg.reply(ctx, &format!("Added {} ({}) to the server list.", name, ip)).await?;

    Ok(())
}

#[command]
async fn remove(ctx: &Context, msg: &Message) -> CommandResult {

    let mut stuff = msg.content.split(" ");
    stuff.next();

    let name = stuff.next();

    if name.is_none() {
        msg.reply(ctx, "Improper command uages. Proper use:\nremove <ServerName>").await?;
        return Ok(());
    }

    let name = name.unwrap();

    let id = &msg.guild_id.unwrap();

    check_guild_server_exists(id).await;

    match SERVERS.write().await.get_mut(id) {
        Some(gs) => {

            if gs.servers.is_empty() {
                msg.channel_id.send_message(&ctx.http, |m| {
                    m.content("There are no saved servers");
                    m
                }).await.unwrap();
                return Ok(());
            }

            let mut ind: Option<usize> = None;

            for (i, s) in gs.servers.iter().enumerate() {
                if s.name.as_ref().unwrap() == name {
                    ind = Some(i);
                    break;
                }
            }

            if ind.is_none() {
                msg.reply(ctx, &format!("There is no saved server with name: {}", name)).await?;
                return Ok(());
            }

            let ind = ind.unwrap();

            if gs.active == ind {
                gs.active = 0;
            } else if gs.active > ind {
                gs.active -= 1;
            }

            let s = gs.servers.remove(ind);

            msg.reply(ctx, &format!("Removed {}", s)).await?;

        },
        None => {
            println!("This guild does not have a record.");
        }
    }

    Ok(())
}

#[command]
async fn setactive(ctx: &Context, msg: &Message) -> CommandResult {

    let mut stuff = msg.content.split(" ");
    stuff.next();

    let name = stuff.next();

    if name.is_none() {
        msg.reply(ctx, "Improper command uages. Proper use:\nsetactive <ServerName>").await?;
        return Ok(());
    }

    let name = name.unwrap().to_string();

    let id = &msg.guild_id.unwrap();

    check_guild_server_exists(id).await;

    let mut servs = SERVERS.write().await;

    match servs.get_mut(id) {
        Some(gs) => {

            let mut ind: Option<usize> = None;

            for (i, s) in gs.servers.iter().enumerate() {
                if s.name.as_ref().unwrap() == &name {
                    ind = Some(i);
                    break;
                }
            }

            if ind.is_none() {
                msg.reply(ctx, &format!("No saved server with name {}", name)).await?;
                return Ok(());
            }

            let ind = ind.unwrap();

            gs.active = ind;

            msg.reply(ctx, &format!("Set active server to {}", gs.servers[gs.active])).await?;


        },
        None => {
            println!("This guild does not have a record.");
        }
    }

    Ok(())
}

#[command]
async fn status(ctx: &Context, msg: &Message) -> CommandResult {

    let mut stuff = msg.content.split(" ");
    stuff.next();

    let name = stuff.next();

    let id = &msg.guild_id.unwrap();

    check_guild_server_exists(id).await;

    match SERVERS.read().await.get(id) {
        Some(gs) => {

            if gs.servers.is_empty() {
                msg.reply(ctx, "There are no saved servers").await?;
                return Ok(());
            }

            match name {
                None => {
                    get_status(ctx, msg, &gs.servers[gs.active]).await?;
                },
                Some(name) => {

                    for s in gs.servers.iter() {
                        if s.name.as_ref().unwrap() == name {
                            get_status(ctx, msg, s).await?;
                            return Ok(());
                        }
                    }

                    msg.reply(ctx, &format!("There is no saved server with name {}", name)).await?;

                }
            }

        },
        None => {
            println!("This Guild has no record.");
        }
    }

    Ok(())
}

#[command]
async fn statusip(ctx: &Context, msg: &Message) -> CommandResult {

    let mut stuff = msg.content.split(" ");
    stuff.next();

    let ip = stuff.next();

    if ip.is_none() {
        msg.reply(ctx, "Improper command uages. Proper use:\nstatusip <ServerIP>").await?;
        return Ok(());
    }

    let ip = ip.unwrap();

    get_status(ctx, msg, &MCServer::new(ip.to_string(), None)).await?;

    Ok(())
}

#[command]
async fn removeall(ctx: &Context, msg: &Message) -> CommandResult {
    let id = &msg.guild_id.unwrap();

    check_guild_server_exists(id).await;

    match SERVERS.write().await.get_mut(id) {
        Some(gs) => {
            gs.active = 0;
            gs.servers.clear();

            msg.reply(ctx, "All servers have been removed!").await?;
        },
        None => {
            println!("This guild does not have a record.");
        }
    }

    Ok(())
}

#[command]
async fn servers(ctx: &Context, msg: &Message) -> CommandResult {

    println!("Here");

    let id = &msg.guild_id.unwrap();

    println!("Got id");

    check_guild_server_exists(id).await;

    println!("Created guild server if there was none");

    match SERVERS.read().await.get(id) {
        Some(gs) => {

            println!("Get guildservers");

            if gs.servers.is_empty() {
                msg.reply(ctx, "There are no saved servers").await?;
                return Ok(());
            }

            msg.channel_id.send_message(&ctx.http, |m| {
                m.embed(|e| {

                    e.title("Servers");

                    e.field("Active", gs.servers[gs.active].to_string(), false);

                    let mut servers = String::new();

                    for s in &gs.servers {
                        servers.push_str(&format!("{}\n", s));
                    }

                    e.field("Saved", servers, false);

                    e
                });
                m
            }).await.unwrap();
        },
        None => {
            println!("This guild does not have a record.");
        },
    }

    Ok(())
}

async fn get_status(ctx: &Context, msg: &Message, serv: &MCServer) -> CommandResult {

    let mut resp = msg.channel_id
            .send_message(&ctx.http, |m| {
                m.embed(|e| {

                    match &serv.name {
                        Some(nam) => {
                            e.title(&format!("{} ({})", nam, &serv.ip));
                        },
                        None => {
                            e.title(&serv.ip);
                        }
                    }
                    e.color(Color::from_rgb(255, 255, 0));
                    e.description("Connecting to server...");

                    e
                });

                m
            }).await?;

            match TcpStream::connect(&serv.ip).await {
                Ok(mut stream) => {

                    resp.edit(&ctx.http, |m| {
                            m.embed(|e| {

                                match &serv.name {
                                    Some(nam) => {
                                        e.title(&format!("{} ({})", nam, &serv.ip));
                                    },
                                    None => {
                                        e.title(&serv.ip);
                                    }
                                }
                                e.color(Color::from_rgb(0, 255, 0));
                                e.description("Connected!");

                                e
                            });

                            m
                        }).await?;


                    match network::status(&mut stream).await {
                        Ok(response) => {

                            let url = String::from("favicon.png");
                            let mut icon: Option<Vec<u8>> = None;

                            resp.delete(ctx).await?;

                            msg.channel_id
                                .send_message(&ctx.http, |m| {


                                // Creat message embed
                                m.embed(|e| {

                                    // Title
                                    match &serv.name {
                                        Some(nam) => {
                                            e.title(&format!("{} ({})", nam, &serv.ip));
                                        },
                                        None => {
                                            e.title(&serv.ip);
                                        }
                                    }

                                    // Extract JSON
                                    match serde_json::from_str::<Value>(&response.response.0) {
                                        Ok(json) => {
                                            if let Value::Object(map) = json {
                                                e.color(Color::from_rgb(0, 255, 0));

                                                // MOTD
                                                if let Some(Value::Object(description)) = map.get("description") {
                                                    if let Some(Value::String(motd)) = description.get("text") {
                                                        e.description(&motd);
                                                    }
                                                }

                                                // Favicon
                                                if let Some(Value::String(favicon)) = map.get("favicon") {

                                                    match base64::decode(&(favicon.replace("\n", ""))[22..]) {
                                                        Ok(bytes) => {
                                                            icon = Some(bytes);

                                                            e.thumbnail(&format!("attachment://{}", url));
                                                        },
                                                        Err(err) => {
                                                            e.description(&format!("Failed to decode favicon: {}", err));
                                                        }
                                                    }

                                                }

                                                // Version number
                                                if let Some(Value::Object(version)) = map.get("version") {
                                                    if let Some(Value::String(ver_num)) = version.get("name") {
                                                        e.field("Version", ver_num, false);
                                                    }
                                                }


                                                // Players
                                                if let Some(Value::Object(players)) = map.get("players") {
                                                    if let Some(Value::Number(max)) = players.get("max") {
                                                        if let Some(Value::Number(online)) = players.get("online") {

                                                                let mut playing = String::from("No players online.");

                                                                if let Some(Value::Array(sample)) = players.get("sample") {
                                                                    for (i, p) in sample.iter().enumerate() {
                                                                        if i == 0 {
                                                                            playing.clear();
                                                                        }
                                                                        if let Value::Object(pp) = p {
                                                                            if let Some(Value::String(name)) = pp.get("name") {
                                                                                playing.push_str(&format!("{}\n", name));
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                e.field(&format!("Players: {}/{}", online, max), playing, false);
                                                        }
                                                    }
                                                }

                                                // Mods (later forge version)
                                                if let Some(Value::Object(forgedata)) = map.get("forgeData") {
                                                    if let Some(Value::Array(mods)) = forgedata.get("mods") {

                                                        let mut mods_str = String::new();

                                                        for mcmod in mods {
                                                            if let Some(Value::String(name)) = mcmod.get("modId") {
                                                                if name == "minecraft" {continue}

                                                                if let Some(Value::String(version)) = mcmod.get("modmarker") {
                                                                    mods_str.push_str(&format!("{} - {}\n", name, version));
                                                                }
                                                            }
                                                        }

                                                        e.field("Mods", mods_str, false);

                                                    }
                                                }

                                                // Mods (earlier forge version)
                                                if let Some(Value::Object(forgedata)) = map.get("modinfo") {
                                                    if let Some(Value::Array(mods)) = forgedata.get("modList") {

                                                        let mut mods_str = String::new();

                                                        for (i, mcmod) in mods.iter().enumerate() {
                                                            if i > 10 {
                                                                mods_str.push_str("And more...");
                                                                break;
                                                            }

                                                            if let Some(Value::String(name)) = mcmod.get("modid") {
                                                                if name == "minecraft" {continue}

                                                                if let Some(Value::String(version)) = mcmod.get("version") {
                                                                    mods_str.push_str(&format!("{} - {}\n", name, version));
                                                                }
                                                            }
                                                        }

                                                        e.field("Mods", mods_str, false);

                                                    }
                                                }

                                            } else {
                                                e.color(Color::from_rgb(255, 0, 0));
                                                e.description("Error interpretting JSON response: Not an Object");
                                            }
                                        },
                                        Err(err) => {
                                            e.color(Color::from_rgb(255, 0, 0));
                                            e.description(&format!("Error interpretting JSON response: {}", err));
                                        }
                                    }
                                    e
                                });

                                // Upload favicon
                                match icon {
                                    None => {},
                                    Some(bytes) => {
                                        m.add_file(AttachmentType::Bytes{
                                            data: Cow::from(bytes),
                                            filename: url,
                                        });
                                    }
                                }

                                m
                            }).await.unwrap();
                        },
                        Err(err) => {
                            resp.edit(ctx, |m| {

                                m.embed(|e| {

                                    e.title("Server name/IP");
                                    e.color(Color::from_rgb(255, 0, 0));
                                    e.description(format!("Failed to retrieve status from server: {}", err));
    
                                    e
                                });

                                m
                            }).await?;
                        },
                    }

                },
                Err(err) => {
                    resp.edit(&ctx.http, |m| {
                            m.embed(|e| {

                                e.title(&format!("{}", serv));
                                e.color(Color::from_rgb(255, 0, 0));
                                e.description(&format!("Couldn't connect to server: {}", err));

                                e
                            });

                            m
                        }).await?;
                }
            }

    Ok(())
}