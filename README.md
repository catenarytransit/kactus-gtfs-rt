# Kactus, a GTFS-rt cache

Kactus (Kyler's Automated Cache for Transport Unification & Synchronisation) is a cache server written in Rust and using Redis. It is open source under the GPL 3.0 license. Please submit issues &
pull requests!

## Install dependencies
arch linux:
```
sudo pacman -S redis protobuf-compiler; sudo systemctl start redis-server;
```
ubuntu:
```
sudo apt install redis protobuf-compiler
sudo systemctl start redis-server
```

### Run the ingest engine
```
cargo run --bin ingest
```
### Hosted by Kyler

Use Kactus hosted by Kyler's servers! 
`https://kactusapi.kylerchin.com/gtfsrt/?feed=[onestopid]&category=[category]`

Check uptime: https://stats.uptimerobot.com/xWx7zCm4p0

Onestop Feed IDs should be realtime feed ids from [transitland/transitland-atlas](https://github.com/transitland/transitland-atlas) aka https://transit.land/

Valid categories are 
- `vehicles` 
- `trips`
- `alerts`

The api returns 404 if the category for the feed doesn't exist.

Example of valid url `https://kactusapi.kylerchin.com/gtfsrt/?feed=f-metro~losangeles~bus~rt&category=vehicles`

### urls.csv config
If the auth_type is set to `url`, then any instance of `PASSWORD` in the urls will be replaced with the value of auth_password
