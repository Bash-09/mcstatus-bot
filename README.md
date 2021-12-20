# mcstatus-bot

A Discord bot to give you a live status update of a Minecraft server in Discord, without even having to launch the game!

![Example of Server Status](images/status.png)

(Please don't judge my code too closely, this was just a small project that turned into something that may be useful to others!)

# Usage

Download the binary [from here](https://github.com/Googe14/mcstatus-bot/releases/latest) and run it with environment variables `DISCORD_TOKEN` and `DISCORD_PREFIX` set to get started.

On Linux devices you can easily create a bash script to run the bot like this:
```
#!/bin/bash

DISCORD_TOKEN=YourDiscordBotTokenHere DISCORD_PREFIX="~" ./MCStatus_Bot
```

I haven't tested it on Windows yet so compile it yourself for now :P

# Features

- Each discord server has their unique list of Minecraft servers, meaning you can host this bot on multiple servers at once without sharing server lists!
- Add/Remove any number of Minecraft servers and give them unique names
- Status update on your set active server with a single `status` command
- Status update on any of your saved servers by using `status` with it's name
- Status update on any minecraft server you want by using `statusip` with it's address

## Status includes:

- Version
- Thumbnail icon
- Max number of players
- Number of online players
- List of which players are online
- Any forge mods the server might include

# Building

Just use Cargo

# Commands

`help` - Opens the help menu\
`add <ServerName> <ServerIP>` - Adds a Minecraft server with a name to the list\
`remove` <ServerName> - Removes a server from the list\
`removeall` - Removes all servers from the list\
`setactive <ServerName>` - Sets a server as the active one so that running status automatically uses that one\
`servers` - Lists all server currently in the list\
`status` - Gets the status of the Minecraft server currently set as active\
`status <ServerName>` - Gets the status of the saved Minecraft server with that name\
`statusip <ServerIP>` - Gets the status of the Minecraft server at that IP, it does not need to be saved for this to work
