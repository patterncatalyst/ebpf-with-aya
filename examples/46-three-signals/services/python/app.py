# Minimal FastAPI service — no OTel SDK. Instrumented from the socket layer.
import asyncio, random
from fastapi import FastAPI

app = FastAPI()

@app.get("/")
async def root():
    await asyncio.sleep(random.random() * 0.03)
    return {"msg": "hello from python"}
