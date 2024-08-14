import sys
import os
from eth_hash.auto import keccak
from cryptography.hazmat.primitives.asymmetric import ed25519
from cryptography.hazmat.primitives import serialization 


def sign_message(msg: str, priv_hex: str):
    priv_key = ed25519.Ed25519PrivateKey.from_private_bytes(bytes.fromhex(priv_hex))
    #print(priv_key.public_key().public_bytes(encoding=serialization.Encoding.Raw, format=serialization.PublicFormat.Raw).hex())
    data = keccak(msg.encode())
    #print("msg.hash", data.hex());
    signature = priv_key.sign(data).hex()
    # print("{} = {}, {}".format(msg, signature, priv_key.public_key().verify(bytes.fromhex(signature), data)))
    return signature

def print_signature(trade_id, buyer, seller, res_addr, volume, price, currency, usd_rate, buyer_fee, seller_fee, payment_method, payee):
    message = "{},{},{},{},{},{},{},{},{},{},{},{}".format(trade_id, buyer, seller, res_addr, volume, price, currency, usd_rate, buyer_fee, seller_fee, payment_method, payee)
    priv_key_hex = os.environ.get("DEXIAN_PRICE_ORACLE_PRIV")
    return sign_message(message, priv_key_hex)


if __name__ == '__main__':
    print(print_signature(sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5], sys.argv[6], sys.argv[7], sys.argv[8], sys.argv[9], sys.argv[10], sys.argv[11], sys.argv[12]))
