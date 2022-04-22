# Generating Media Assets

## Install dependencies

- <https://github.com/asciinema/asciinema>
- <https://github.com/asciinema/asciicast2gif>

## Record a new asciicast

```console
$ asciinema rec -i .3 -c bash media/qparas.cast
  # <inside recording sesson>
$ qparas token-series collection_id=mint.havendao.near min_price=0 __sort=metadata.score::-1
```

## Convert your asciicast to gif

```console
asciicast2gif -t tango -S 3 media/qparas.cast media/demo.gif
```
