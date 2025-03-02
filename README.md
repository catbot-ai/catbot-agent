# catbot-agent

![image](https://github.com/user-attachments/assets/5349d160-1519-4504-be69-02c0505fe5cc)

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
npx wrangler secret put GEMINI_API_KEY
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

```

## Features

### Free

- See yesterday tab result via web.
- See today tab blurry result with `stake` button via web.

### Staked

- See yesterday tab result via web.
- See today tab result via web.
- See next hour tab. (more stake more hour)

## Rules

- [free-email] user get end of the day summary, link to website.
- [free-web] user can see yesterday result.
- [free-web] user can see profit comparison between `free` | `signed-in` | `staked`.
- Stake to get signals.
- More stake = more update frequency.
- Update via email and discord.
- Unstake took 7 days.
- Staker get role in Discord.
- Gold member get access to the bot.
```
