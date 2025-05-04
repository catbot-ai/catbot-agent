# Feeder

## Setup (DONE)

```
npx wrangler kv namespace create ASSETS --preview
npx wrangler kv namespace create ASSETS

npx wrangler kv key put --binding=ASSETS "RobotoMono-Regular.ttf" --path=RobotoMono-Regular.ttf --local --preview
npx wrangler kv key put --binding=ASSETS "RobotoMono-Regular.ttf" --path=RobotoMono-Regular.ttf --preview false
```

## Develop (local)

```
# local
npx wrangler dev --live-reload --port 9090
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
