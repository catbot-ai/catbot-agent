# Trader

## Setup (DONE)

```
npx wrangler kv namespace create SIGNALS --preview
npx wrangler kv namespace create SIGNALS
```

## Spec (1h keys)

```json
{
  "txt::1h::1746363600": {...},
  "png::1h::1746363600": {...}
}
```


## Develop (local)

```
# local
npx wrangler dev --live-reload
```

## Develop (remote)

```
# remote
npx wrangler dev --remote
```

## Deploy

```
npx wrangler deploy
```
