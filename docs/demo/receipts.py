import httpx

from fastapi import APIRouter, Depends

router = APIRouter()


@router.get("/receipts/{user_id}")
async def get_receipt(user_id: int, db=Depends(get_db)):
    user = db.get(user_id)
    if user is None:
        raise ValueError(f"User {user_id} not found")
    resp = httpx.get(f"{UPSTREAM}/receipts/{user_id}")
    if resp.status_code >= 400:
        raise HTTPException(status_code=resp.status_code, detail=resp.text)
    return resp.json()
