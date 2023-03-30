from datetime import datetime

from Crypto.Hash import SHA256
from solana.publickey import PublicKey
from base64 import b64encode, b64decode 
from base58 import b58encode, b58decode
from construct import Struct, BytesInteger, Bytes, Adapter, this, PascalString
from construct import setGlobalPrintFullStrings


setGlobalPrintFullStrings(True)


class PubkeyAdapter(Adapter):
    def _decode(self, obj, context, path):
        return PublicKey(obj)
    def _encode(self, obj, context, path):
        return bytes(obj)


class TimestampAdapter(Adapter):
    def _decode(self, obj, context, path):
        return datetime.fromtimestamp(obj)
    def _encode(self, obj, context, path):
        return obj.timestamp()


launch_schema = Struct(
    Bytes(8),
    "seller" / PubkeyAdapter(Bytes(32)),
    "mint" / PubkeyAdapter(Bytes(32)),
    "price" / BytesInteger(8, swapped=True),
    "rate" / BytesInteger(2, swapped=True),
    "label" / PascalString(BytesInteger(4, swapped=True), "utf8"),
)

redeem_schema = Struct(
    Bytes(8),
    "redeem" / PubkeyAdapter(Bytes(32)),
    "mint" / PubkeyAdapter(Bytes(32)),
    "label" / PascalString(BytesInteger(4, swapped=True), "utf8"),
)

sold_schema = Struct(
    Bytes(8),
    "seller" / PubkeyAdapter(Bytes(32)),
    "mint" / PubkeyAdapter(Bytes(32)),
    "customer" / PubkeyAdapter(Bytes(32)),
    "index" / BytesInteger(4, swapped=True),
    "price" / BytesInteger(8, swapped=True),
    "rate" / BytesInteger(2, swapped=True),
    "created_at" / BytesInteger(8, signed=True, swapped=True),
    "label" / PascalString(BytesInteger(4, swapped=True), "utf8"),
)


def main():
    h = SHA256.new()
    h.update(b"global:buy_nft")
    buy_instruction = h.digest()
    print("buy: ", buy_instruction.hex()[:8])

    h = SHA256.new()
    h.update(b"global:redeem_nft")
    redeem_instruction = h.digest()
    print("redeem: ", redeem_instruction.hex()[:8])

    h = SHA256.new()
    h.update(b"global:sell_nft")
    sell_instruction = h.digest()
    print("sell: ", sell_instruction.hex()[:8])

    launch_raw_data = b64decode("G8EvgnNc714ghJPIGDHbCdrgrU8MaRls7X/61QZUGCgF3rCP0VrtCzZTz61fbhUnkepBBuUD4OPljk7pV652PcbzojBREKekALEIGQAAAABkAAgAAABzZWxsX25mdA==")
    print(launch_schema.parse(launch_raw_data))

    redeem_raw_data = b64decode("WnJTktQa2Ts/vLHSJb+T+d7jDEefrskVDZwMvcBr6hXC9ESTmrkg8bEy9JAXavx9mV32ehcU3t1x8LxpXejE7PWsjQRNbYU8CgAAAHJlZGVlbV9uZnQ=")
    print(redeem_schema.parse(redeem_raw_data))

    sold_raw_data = b64decode('XB5FWdAhrx1DU47UFrGnHMTsb0EaE1TBoVQGvCIHKJ4/EvpK3zvIf075pStcj61MRI3yjgnU0cKJYzZhgJnTi1PEiOoPUGwRfML+FaRXPyI7hVvuAh0wHq+J4s/oPSji5eSb8qjsBSkAAAAAAMqaOwAAAAABAJmOuWEAAAAABwAAAGJ1eV9uZnQ=')
    print(sold_schema.parse(sold_raw_data))



if __name__ == "__main__":
    main()
