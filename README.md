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
cargo run --bin ingestv2
```
### Install Systemd Service
```bash
sudo cp systemd* /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now kactua* zotgtfsrt.service
```

### Hosted by Kyler

Use Kactus hosted by Kyler's servers! 
`https://kactus.catenarymaps.org/gtfsrt/?feed=[onestopid]&category=[category]`

Check uptime: https://stats.uptimerobot.com/xWx7zCm4p0

Onestop Feed IDs should be realtime feed ids from [transitland/transitland-atlas](https://github.com/transitland/transitland-atlas) aka https://transit.land/

Valid categories are 
- `vehicles` 
- `trips`
- `alerts`

The api returns 404 if the category for the feed doesn't exist.

Example of valid url `https://kactus.catenarymaps.org/gtfsrt/?feed=f-metro~losangeles~bus~rt&category=vehicles`

### urls.csv config
If the auth_type is set to `url`, then any instance of `PASSWORD` in the urls will be replaced with the value of auth_password

### options for ingest file

you can specify the urls file to use, but by default, it is `urls.csv`
```bash
cargo run --bin ingestv2 -- --urls public-urls.csv
```

you can add the `threads` parameter

```bash
cargo run --bin ingestv2 -- --urls public-urls.csv --threads 10
```

you can also add the timeout parameter in milliseconds, the default being `15000` ms aka 15 seconds.
```bash
---timeout 10000
```

### Run the special metrolink program using Kyler's "Request in Perfect Time" algorithm

Basically this skirts around the 429 rate limit error by sending requests almost exactly 30 seconds after the last request.

First, paste your metrolink key into the file `metrolink-key.txt`

Then run
```bash
cargo run --bin ingestmetrolink
```
