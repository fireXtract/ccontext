# Copy Context (Menu)

For people that must use windows, but also have a NAS that has docker containers 
in it, and are sick of getting the path another way. 

Places additional windows right click entries when in the specified
network mapped drive.


## Configuration
Config supposes when you navigate to `L:\` on the windows host you see dir `a` 
(meaning samba drops you into `/media/`), and that you have a docker container
on that remote where `a/iso` is mounted to `/data`

``` docker-compose.yaml
  my-service:
    volumes:
      - /media/a/isos:/data
```

In the likely case that this is NOT exactly the setup you have, modify
ccontext_install.reg accordingly. Don't forget to update the path to the binary as well, since its unlikely you place your Rust projects in a directory named java like I do.

For my use case it's configured as Windows, Remote, and Docker. You could get
creative in the .reg since it's all arg based
ex. cygwin, git bash, a local docker

I'm chasing minimal binary size, so for now CopyPathAs.Docker will behave wrongly
if the file you pick doesn't exist below that path.

### https://github.com/johnthagen/min-sized-rust
24KB when built with build_tiny.sh
113KB with just `cargo build --release` which upx brings down to 54KB
To go much smaller I'd have to do my own C string formatting, which doesn't sound fun, and it'd probably be wiser to 
just write C at that point.

#### I have rust installed on Windows, but use cygwin to run bash scripts with windows binaries because I refuse to learn PowerShell.
