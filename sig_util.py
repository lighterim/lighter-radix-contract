import sys
import os
from eth_hash.auto import keccak
from cryptography.hazmat.primitives.asymmetric import ed25519


def sign_message(msg: str, priv_hex: str):
    priv_key = ed25519.Ed25519PrivateKey.from_private_bytes(bytes.fromhex(priv_hex))
    #print(msg)
    data = keccak(msg.encode())
    signature = priv_key.sign(data).hex()
    # print("{} = {}, {}".format(msg, signature, priv_key.public_key().verify(bytes.fromhex(signature), data)))
    return signature

def print_signature(trade_id, buyer, seller, res_addr, volume, price, buyer_fee, seller_fee, payment_method):
    message = "{},{},{},{},{},{},{},{},{}".format(trade_id, buyer, seller, res_addr, volume, price, buyer_fee, seller_fee, payment_method)
    priv_key_hex = os.environ.get("DEXIAN_PRICE_ORACLE_PRIV")
    return sign_message(message, priv_key_hex)


if __name__ == '__main__':
    print(print_signature(sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5], sys.argv[6], sys.argv[7], sys.argv[8], sys.argv[9]))
