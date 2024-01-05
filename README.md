# Kactus, a GTFS-rt cache

Kactus (Kyler's Automated Cache for Transport Unification & Synchronisation) is a cache server written in Rust and using Redis. It is open source under the GPL 3.0 license. Please submit issues &
pull requests!

### Hosted by Catenary Transit Initatives Data Centres

Use Kactus hosted by Catenary's servers! 
`https://kactus.catenarymaps.org/gtfsrt/?feed=[onestopid]&category=[category]`

Check uptime: https://stats.uptimerobot.com/xWx7zCm4p0

Onestop Feed IDs should be realtime feed ids from [transitland/transitland-atlas](https://github.com/transitland/transitland-atlas) aka https://transit.land/

Valid categories are 
- `vehicles` 
- `trips`
- `alerts`

The api returns 404 if the category for the feed doesn't exist.

Example of valid url `https://kactus.catenarymaps.org/gtfsrt/?feed=f-metro~losangeles~bus~rt&category=vehicles`

#### Knowing valid feeds and categories

The list of avaliable feeds is at `https://kactus.catenarymaps.org/gtfsrttimes`

#### Debugging by hand
`https://kactus.catenarymaps.org/gtfsrtasjson/?feed=[onestopid]&category=[category]`

or use raw for Rust-info
`https://kactus.catenarymaps.org/gtfsrtasjson/?feed=[onestopid]&category=[category]&raw=true`
like so
`https://kactus.catenarymaps.org/gtfsrtasjson/?feed=f-metro~losangeles~bus~rt&category=vehicles&raw=true`

# Installation


## Install dependencies
arch linux:
```
sudo pacman -S redis protobuf-compiler; sudo systemctl start redis-server;
```
ubuntu:
```
sudo apt install protobuf-compiler build-essential gcc pkg-config libssl-dev postgresql unzip wget
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
sudo systemctl enable --now kactus* zotgtfsrt.service
```

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

## For Contributors

For unix users, running `git config core.hooksPath .githooks` is required.
Pull requests will not be merged without this.

No option exists for Windows users at the moment. Please try WSL Ubuntu for the moment. We're working on adding this.
