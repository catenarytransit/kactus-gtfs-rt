# Kactus, a GTFS-rt cache

Kactus (Kyler's Automated Cache for Transport Unification & Synchronisation) is a cache server written in Rust and using Redis. It is open source under the GPL 3.0 license. Please submit issues &
pull requests!

## Install dependencies
arch linux:
```
sudo pacman -S redis; sudo systemctl start redis-server;
```
ubuntu:
```
sudo apt install redis
sudo systemctl start redis-server
```

### Run the ingest engine
```
cargo run --bin ingest
```
### Hosted by Kyler

Use Kactus hosted by Kyler's servers! (NOT READY YET, DO NOT USE YET)
`api.kactus.kylerchin.com/`

### urls.csv config
If the auth_type is set to `url`, then any instance of `PASSWORD` in the urls will be replaced with the value of auth_password
