# catbot-agent

![image](https://github.com/user-attachments/assets/59e64208-30a8-41dd-980e-b0754e2d7645)

Let's llm do the things!

## Dev

```
cd cooker
npx wrangler dev
```

## Deploy

```
npx wrangler deploy
```

## Secret

```
# cooker
npx wrangler secret put GEMINI_API_KEY

# feeder
npx wrangler secret put PREDICTION_API_URL
https://catbot-cooker.foxfox.workers.dev/api/v1/predict
```

## TODO

- Get vibe from x.
- Post to discord
- Post to x
- Get graph with indicator

  ```
  curl -X 'POST' 'https://api.cloudflare.com/client/v4/accounts/7e11517c4dd4f6e9cede7da9b60d66eb/browser-rendering/screenshot' \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer KEY' \
  -d '{
  "url": "https://www.binance.com/en/trade/SOL_USDT",
      "viewport": {
      "width": 1640,
      "height": 960
      },
      "gotoOptions": {
      "waitUntil": "networkidle2",
      "timeout": 30000
      }
  }' \
  --output "screenshot.webp"
  ```

## Flow

```mermaid
graph TD;
  I(⏱️ cranker)--"1m trigger"--> A
  AA(🌼 binance)--"token prices"-->A
  BB("🍰 tasks")<--"signals<br>actions<br>positions"-->O("🤖 vlm_actor")
  A(🐝 cooker)--"chart 15m,1h,4h<br>ema,bb,mcad<br>sum_signals"-->B("🍯 signals")--"chart 15m,1h,4h<br>ema,bb,mcad<br>sum_signals"-->C("🤖 llm_feeder")
  C--"sum_signals<br>text+chart"-->BBB("🎂 reranked_signals")
  BBB-->O
  O--actions-->AB--"positions"-->O
  AB(🌸 jupiter)--"token prices<br>positions"-->A
```

## Features

### Free

- See yesterday tab result via web.
- See today tab blurry result with `stake` button via web.
- Only one token.

### Staked

- See yesterday tab result via web.
- See today tab result via web.
- More token.
- See rebalance tab.
- Unstake took 7 days.
- Get role in Discord.
- Get circuit breaker signal.

## TODO

- [ ] Try vlm with graph, create struct from extracted value.
- [ ] Watch for 500k volume via websocket.
- [ ] Trigger prediction every 5 minute.
- [ ] Store signals in KV/DO.
