# rtime

Similar to Linux `time` command but writing the elapsed time continuously on screen

To use it

```sh
cargo install rtime
```

Example

```sh
rtime 'ls; sleep 3; echo "sadfasdfasdfasdfa fasdf asdf asdf asd fasd asd f asdf asdfsadf asdf "; sleep 4'
```

![Example](/rtime_ex.png)

Example with stdout and stderr

```sh
rtime 'echo 1; echo 2; >&2 echo  3; >&2 echo 4; echo 5; echo 6; echo 7; echo 8'
```
