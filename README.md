# ColorMe-Cmd
This is an example application in `rust` which, given a configuraiton of
processes to run/tail it will spawn them and output each one in a different
color if the lines contain a specified regex pattern.

for example, this config:

```yaml
commands:
  - bin: "ls"
    args: ["/dev", "-ltr"]
    out_color: purple 

  - bin: "tail"
    args: ["-F", "/var/log/mpd/mpd.log"]
    out_color: green 

  - bin: "tail"
    args: ["-F", "/var/log/syslog"]
    out_color: blue 
    filter: "usb"
```

lists out my `/dev` directory and then will print updates from my music player daemon logs in green, while overlaying syslog messages that contain "usb" in blue so that I can see the two pieces of information together.

It is intended for debugging multiple applications where you are looking for a specific phrase and would like the diffent applications to show up in different colors.

It is an example piece, and as such may have a few rough spots.
