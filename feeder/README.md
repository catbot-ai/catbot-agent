# Feeder

## Setup (DONE)

```
npx wrangler kv namespace create ASSETS --preview
npx wrangler kv namespace create ASSETS

npx wrangler kv key put --binding=ASSETS "Roboto-Light.ttf" --path=Roboto-Light.ttf --local --preview
npx wrangler kv key put --binding=ASSETS "Roboto-Light.ttf" --path=Roboto-Light.ttf --preview false
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
