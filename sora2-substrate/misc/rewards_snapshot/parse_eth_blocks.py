# This file is part of the SORA network and Polkaswap app.

# Copyright (c) 2020, 2021, Polka Biome Ltd. All rights reserved.
# SPDX-License-Identifier: BSD-4-Clause

# Redistribution and use in source and binary forms, with or without modification, 
# are permitted provided that the following conditions are met:

# Redistributions of source code must retain the above copyright notice, this list 
# of conditions and the following disclaimer.
# Redistributions in binary form must reproduce the above copyright notice, this 
# list of conditions and the following disclaimer in the documentation and/or other 
# materials provided with the distribution.
# 
# All advertising materials mentioning features or use of this software must display 
# the following acknowledgement: This product includes software developed by Polka Biome
# Ltd., SORA, and Polkaswap.
# 
# Neither the name of the Polka Biome Ltd. nor the names of its contributors may be used 
# to endorse or promote products derived from this software without specific prior written permission.

# THIS SOFTWARE IS PROVIDED BY Polka Biome Ltd. AS IS AND ANY EXPRESS OR IMPLIED WARRANTIES, 
# INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR 
# A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL Polka Biome Ltd. BE LIABLE FOR ANY 
# DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, 
# BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; 
# OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, 
# STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE 
# USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

from eth_abi import decode_single
from eth_bloom import BloomFilter
from web3 import Web3
from web3.exceptions import BlockNotFound, TransactionNotFound
import websockets
import sys
import time

attempt_count = 3
contract = '0xd07dc4262bcdbf85190c01c996b4c06a461d2430'
contract_bytes = Web3.toBytes(hexstr=contract)
buy_topic = Web3.keccak(
    text='Buy(address,uint256,address,uint256,address,uint256)')
transfer_single_topic = Web3.keccak(
    text='TransferSingle(address,address,address,uint256,uint256)')
transfer_batch_topic = Web3.keccak(
    text='TransferBatch(address,address,address,uint256[],uint256[])')
approval_for_all_topic = Web3.keccak(
    text='ApprovalForAll(address,address,bool)')
supported_tokens = [12277, 30297, 6929, 24403, 77235, 88849, 6895, 112043]

def create_w3():
    return Web3(Web3.WebsocketProvider('ws://127.0.0.1:8546', websocket_timeout=600))


def get_block(w3, block_number):
    attempt = 1
    while True:
        try:
            return (w3, w3.eth.get_block(block_number))
        except BlockNotFound:
            return (w3, None)
        except websockets.exceptions.ConnectionClosedError:
            if attempt != attempt_count:
                print(f'attempt {attempt} failed. connection is closed when trying to get block {block_number}. sleeping')
                time.sleep(60)
                attempt += 1
                w3 = create_w3()
            else:
                raise


def get_transaction_receipt(w3, tx_hash):
    attempt = 1
    while True:
        try:
            return (w3, w3.eth.getTransactionReceipt(tx_hash))
        except:
            if attempt != attempt_count:
                print(
                    f'attempt {attempt} failed. connection is closed when trying to get transaction receipt {tx_hash}. sleeping')
                time.sleep(60)
                attempt += 1
                w3 = create_w3()
            else:
                raise


def handle_transaction(w3, tx_hash, flow, unhandled_transactions):
    w3, receipt = get_transaction_receipt(w3, tx_hash)
    if receipt is None:
        return (w3, None)
    if receipt.status == 0:
        return (w3, {})
    transaction_owners = {}
    handled = False
    for log in receipt.logs:
        if log.address.lower() != contract.lower():
            continue
        if log.topics[0] == transfer_single_topic or log.topics[0] == transfer_batch_topic:
            if log.topics[0] == transfer_single_topic:
                token, qty = decode_single(
                    '(uint256,uint256)', Web3.toBytes(hexstr=log.data))
                tokens = [token]
                qtys = [qty]
            else:
                tokens, qtys = decode_single(
                    '(uint256[],uint256[])', Web3.toBytes(hexstr=log.data))
            giver = log.topics[2].hex()
            taker = log.topics[3].hex()
            if giver == '0x0000000000000000000000000000000000000000':
                if taker not in flow:
                    flow[taker] = {}
                if 'minted' not in flow[taker]:
                    flow[taker]['minted'] = []
                flow[taker]['minted'].append((tx_hash, tokens, qtys))
            elif taker == '0x0000000000000000000000000000000000000000':
                if giver not in flow:
                    flow[giver] = {}
                if 'burned' not in flow[giver]:
                    flow[giver]['burned'] = []
                flow[giver]['burned'].append((tx_hash, tokens, qtys))
            else:
                if giver not in flow:
                    flow[giver] = {}
                if 'sent' not in flow[giver]:
                    flow[giver]['sent'] = []
                flow[giver]['sent'].append((tx_hash, taker, tokens, qtys))

                if taker not in flow:
                    flow[taker] = {}
                if 'received' not in flow[taker]:
                    flow[taker]['received'] = []
                flow[taker]['received'].append((tx_hash, giver, tokens, qtys))
            for token, qty in zip(tokens, qtys):
                if token in supported_tokens:
                    if giver not in transaction_owners:
                        transaction_owners[giver] = {token: -qty}
                    elif token not in transaction_owners[giver]:
                        transaction_owners[giver][token] = -qty
                    else:
                        transaction_owners[giver][token] -= qty

                    if taker not in transaction_owners:
                        transaction_owners[taker] = {token: qty}
                    elif token not in transaction_owners[taker]:
                        transaction_owners[taker][token] = qty
                    else:
                        transaction_owners[taker][token] += qty
            handled = True
        elif log.topics[0] == approval_for_all_topic:
            # owner = log.topics[1].hex().replace('000000000000000000000000', '')
            # operator = log.topics[2].hex().replace(
            #     '000000000000000000000000', '')
            # approved = decode_single('bool', Web3.toBytes(hexstr=log.data))
            # if approved:
            #     print(f'{owner} approved {operator}')
            # else:
            #     print(f'{owner} disapproved {operator}')
            handled = True
    if not handled:
        unhandled_transactions.append(tx_hash)
    return (w3, transaction_owners)


def merge_owners(target, source):
    for owner, tokens in source.items():
        if owner not in target:
            target[owner] = tokens
        else:
            for token, qty in tokens.items():
                if token not in target[owner]:
                    target[owner][token] = qty
                else:
                    target[owner][token] += qty


def normalize_owners(owners):
    owner_keys = list(owners.keys())
    for owner in owner_keys:
        tokens = owners[owner]
        token_keys = list(tokens.keys())
        for token in token_keys:
            if tokens[token] == 0:
                tokens.pop(token)
        if len(tokens) == 0:
            owners.pop(owner)

w3 = create_w3()
block_number = 12146082
owners = {
    "0x000000000000000000000000cd434c753f9c27960a411e6e48a74ed7f38287f4": {6895: 1},
    "0x000000000000000000000000ac1dc457ae74ab701635bfd5ff1d13a4ff110947": {6895: 1},
    "0x00000000000000000000000036fb704c0bb0b3e2db258eeb33fc3757fe92b564": {
        6895: 7,
        6929: 29,
    },
    "0x000000000000000000000000896b94f4f27f12369698c302e2049cae86936bbb": {6929: 10},
    "0x00000000000000000000000093207b8c861cf2b96a92c41d99da0e9615f1a3ea": {6929: 1295},
    "0x000000000000000000000000490697b59520cfa81938df18b4634081556d3d15": {6929: 1},
    "0x000000000000000000000000d3f714357a3a5ab971941ca475a006814edd0d2b": {6929: 2},
    "0x00000000000000000000000042147ee918238fdff257a15fa758944d6b870b6a": {
        24403: 2,
        30297: 1,
    },
    "0x000000000000000000000000f0c82a44ba3503f86f2b2772cb5038999a24c7e0": {6929: 100},
    "0x000000000000000000000000bf3c74d8efd7646db36df32c4ee518ac9e1237a4": {6929: 250},
    "0x000000000000000000000000f8a9ec4d9223b420aab0b7de3f2ddeff16ff9b6d": {6929: 1},
    "0x000000000000000000000000f4583ac5c12df365a850fe2f9ff63dc7acbaffcb": {6929: 2},
    "0x0000000000000000000000004e04a5320ca2e6278c5192136e6992659dcf4607": {6929: 7},
    "0x00000000000000000000000071ed1ed34474469f09622377801da0a35363d2d7": {6929: 40},
    "0x0000000000000000000000001948f950899cc5213ef9ff7e543afbda16f86de6": {6929: 16},
    "0x00000000000000000000000081bf263a0e35a75fd5e10ae6a1ed71a5c335e19c": {6929: 2},
    "0x000000000000000000000000339df4db958e19ad3b642baad7ad7f7dc3900ed5": {
        6929: 257,
        12277: 1,
    },
    "0x00000000000000000000000098868b2f8104f29815d72bdbb667fd7753b829f7": {6929: 3},
    "0x000000000000000000000000e84dced112f61f1dabc900e928114168eef8d820": {12277: 1},
    "0x0000000000000000000000009be6a134b409b718a19bc895d5d5939ae0e24ba8": {6929: 1},
    "0x000000000000000000000000e742b9592a928beb809fe6ec2af0c837283e747f": {6929: 29},
    "0x000000000000000000000000b0c8fce2dc46e0e0373dda24d20ef9fe4a7d4466": {6929: 1},
    "0x000000000000000000000000e81226e4308ea4ab62fbd735965884f2014e00d2": {6929: 1},
    "0x00000000000000000000000052f658f09f78bd8153beae59fb3099c2fb629fc2": {6929: 1},
    "0x0000000000000000000000006941f45c846955cc077e459ab283e21b5a85960b": {6929: 14},
    "0x00000000000000000000000025c7257f7acb51f1c409d3ac6fcbe4004d72d34f": {6929: 2},
    "0x000000000000000000000000e594ce3f5a54b5b263b5c7e5a3e2c46bac13c239": {6929: 33},
    "0x00000000000000000000000072dd07903aee4f03697a3d585f21f7e107f4e6c5": {6929: 9},
    "0x000000000000000000000000875fb69aab3d9314198f705288d3ed29731b7b21": {6929: 1},
    "0x0000000000000000000000007ca4db5b87522561af454617a9e82955c7791c20": {12277: 1},
    "0x000000000000000000000000f8283d5f9ea151dd6710f7558de94d6cfba6198d": {6929: 203},
    "0x000000000000000000000000574a782a00dd152d98ff85104f723575d870698e": {6929: 1},
    "0x0000000000000000000000000fd95c6aab9e21b79eeaeb78e6b13de93305f8c0": {6929: 2},
    "0x000000000000000000000000f983557ec70fbf1a4b1e247af7bf10247e9b69c4": {12277: 2},
    "0x00000000000000000000000015838bde57f6d5b7a9fa20de83a4b00f0b1961bd": {6929: 2},
    "0x00000000000000000000000032c26f7a88768d1a8eb42f24c7ebb08c22242cc1": {6929: 1},
    "0x00000000000000000000000004f2290215b18b79a5ddf2e98ee983755dd7a645": {6929: 26},
    "0x0000000000000000000000002af8a3e33fb6da49881080e397dabeb06162f3d1": {6929: 8},
    "0x00000000000000000000000005913a70848be8964523a029b834d324a737c17a": {6929: 195},
    "0x0000000000000000000000009eafa757cbeef9e5236e4a90ca11fc1ebda69507": {6929: 2},
    "0x000000000000000000000000d2ed06a59c33a08c5c81d47825b255cc87018c66": {6929: 35},
    "0x0000000000000000000000007ce8a1690f8fa09a4b3c3eb2b9b21ef9560b9278": {6929: 1},
    "0x000000000000000000000000879c2ee5adc372d0542d65d80ebfe647ac1a5f10": {6929: 4049},
    "0x000000000000000000000000b837b82d7bd9e4d5feefcc13bb9d72b257b87ee9": {6929: 1},
    "0x0000000000000000000000009f12587b7114c57d9d6584853447b70ad0c4b6f1": {6929: 3},
    "0x000000000000000000000000f42a339f93c1fa4c5d9ace33db308a504e7b0bde": {6929: 2},
    "0x000000000000000000000000b67d92dc830f1a24e4bffd1a6794fcf8f497c7de": {6929: 1},
    "0x000000000000000000000000b1b4fc64f3b9657fd7e1a4fe39801ab0de134de9": {6929: 1},
    "0x00000000000000000000000039979745b166572c25b4c7e4e0939c9298efe79d": {6929: 2},
    "0x0000000000000000000000006d60173f4ddeacff28f2ce9fbc21b75f5c210484": {6929: 44},
    "0x00000000000000000000000002c1617d7bb53f51c5e3bc38fc02432e38020152": {6929: 2},
    "0x00000000000000000000000085669d7f1e9fed106f8bc14df86869ebfd33d7b2": {6929: 36},
    "0x000000000000000000000000345b47bfa3d61b8826a1fb4ac6f4c18cd15a6079": {6929: 2},
    "0x0000000000000000000000009bcbfe550d32dfdd2d047ca52f497cba1f564b6b": {
        6929: 10,
        24403: 1,
    },
    "0x000000000000000000000000c85b20025b4c3b3b40fabfb53f643e97f4b234f6": {6929: 3},
    "0x0000000000000000000000009cd0b5f7165f80f71b307314949208e1a94beabb": {6929: 1},
    "0x0000000000000000000000005e7a1573620e0df38e41dd302f68d7d8e5b99bba": {6929: 1},
    "0x000000000000000000000000b8d7b045d299c9c356bc5ee4fe2dddc8a31280a5": {6929: 1},
    "0x000000000000000000000000fcad3475520fb54fc95305a6549a79170da8b7c0": {6929: 1},
    "0x00000000000000000000000056de04c8ad6a72b44c32a4dd984b211698a93767": {6929: 1},
    "0x00000000000000000000000068b55927cc8fced426bcb8ecafdfd224eabc0340": {6929: 5},
    "0x0000000000000000000000003631a6b1318a73dd2dbf890713a8aaa2c98c9a50": {6929: 2},
    "0x00000000000000000000000045f7c1b3e66e5936bfd3834effed93c82d8d069c": {6929: 1},
    "0x000000000000000000000000b710466853f55bc359b82141ae79ed9e6afb48db": {6929: 1},
    "0x0000000000000000000000003a6c03cbcbbb21aabd63045015dd123338101a00": {6929: 1},
    "0x0000000000000000000000005a9e1c0fa76916f1253528db09a6f6f451f4431d": {6929: 2},
    "0x0000000000000000000000004e2829d38433eb3f36057d48cb73a0f3d7abb9be": {6929: 4},
    "0x0000000000000000000000006ee5e150afcf8d1dc4c80a97b7e1abe54210ca9d": {6929: 100},
    "0x0000000000000000000000002416f8e669e667f276474c3c052e613eb1d17e8c": {6929: 154},
    "0x000000000000000000000000d5bd6f24f6983c91005eaf1a230b659aedd2bdff": {6929: 1},
    "0x000000000000000000000000ebf6fc6c7f8f97db7755f0938ce71a95059c59d7": {6929: 19},
    "0x00000000000000000000000056afc85e897189d992d62b30d0b707806ace45fe": {6929: 3},
    "0x000000000000000000000000208f6f2e922168851fbc42ea9078da202888a79e": {6929: 9},
    "0x0000000000000000000000001d32ec413af47d872ea957baf09a0887b00eb470": {6929: 6},
    "0x000000000000000000000000dda82d906813d263017715d3f199510794889210": {6929: 1},
    "0x000000000000000000000000475c3edf728712510b2cadf65207254e18ee5134": {6929: 1},
    "0x0000000000000000000000009759cd43042bb2ce7ba22d3e2beb675153442d80": {6929: 3},
    "0x000000000000000000000000c902216363b76378a4ddd662776526aa8f34acf9": {6929: 2},
    "0x000000000000000000000000e9e8a9c7e715fb0a4b62729dbbf2301f946524ca": {6929: 12},
    "0x000000000000000000000000d1b572f9528b70df1ea79456edc8250125f2d6bb": {6929: 4},
    "0x000000000000000000000000bc664ae7b98015b1f5aaa7fa3a68322278f33f94": {6929: 576},
    "0x0000000000000000000000003e6415833dfe5a4884f6f6904f76f15d62295854": {6929: 1},
    "0x000000000000000000000000c97c2905b706eb7a69094e0d0ed7986b34df9d25": {6929: 1},
    "0x00000000000000000000000021ffcb394232ede3533c8d73328b3cb4f9a60406": {6929: 10},
    "0x000000000000000000000000bb403d793dda52f8512950953771b7c341086120": {6929: 1},
    "0x00000000000000000000000076f168cb327dd991c8291006d5ccb946d3cf5d6d": {6929: 2},
    "0x000000000000000000000000fe60dd65443f5abe76aa5dc3a76043309068cb4a": {6929: 1},
    "0x0000000000000000000000004bec0c9b41343773c6a36e8f8136bf06019d00a5": {6929: 2},
    "0x000000000000000000000000a2a76515fe779badd0ba5db3f62c51c14d2a70b8": {6929: 1},
    "0x0000000000000000000000000998160bdf3ff6d86a4e9d5c31e0efc3ca7e7d01": {6929: 4},
    "0x00000000000000000000000099391c6f4d33ddac56e0856db4ef0013851031bd": {6929: 8},
    "0x000000000000000000000000292b78a5ad6214971c0ec79cb9d7eb3cf20957fb": {6929: 3},
    "0x000000000000000000000000ddf767f258adf0af89896621349cadcf8722f771": {6929: 2},
    "0x00000000000000000000000069fe2badd12f4515aaf99e3a9956b9ffae56f877": {6929: 7},
    "0x0000000000000000000000006fd3ba02d46f0cd64dc40dc945d2229ea5824255": {6929: 1},
    "0x0000000000000000000000002c79fa060c0267bd2f652cdc7e06cef0a9234b3b": {6929: 3},
    "0x0000000000000000000000008d2d6f4d257c177af8111a084bf18a77047e1596": {6929: 1},
    "0x000000000000000000000000515ab913d8d9e984d518dbb59adc02067da7fbad": {6929: 1},
    "0x000000000000000000000000f015bede6e33d12b3f36dd99d0adba7a90086de2": {
        6929: 1,
        12277: 10,
        24403: 1,
        30297: 1,
        77235: 1,
    },
    "0x000000000000000000000000c105a532f5a6af371f916de309f921b48658d3de": {6929: 1},
    "0x000000000000000000000000a0c0b4788a8fd6c2e1c7b7c503040d696412f0f6": {30297: 10},
    "0x000000000000000000000000ca19d045317224c3bb84af1a403f70bd3589ef56": {12277: 1},
    "0x000000000000000000000000714a644c698a52784f74f0a5b46cd58b4a48fc58": {12277: 1},
    "0x00000000000000000000000021271500be1638e05e6fba300520f4f019f34a4e": {6929: 6},
    "0x00000000000000000000000031381b746fad1fb60fcd7c092fa45be14dd6fd13": {12277: 1},
    "0x0000000000000000000000008ca7e48255f3dda02efc7ded4cbaca3490397d90": {12277: 148},
    "0x0000000000000000000000006cf27aa96ea166bfa973b8bafffe7856d12d99e9": {12277: 350},
    "0x000000000000000000000000231724c0064c25387a6ca81aa043c3e90676975b": {6929: 3},
    "0x000000000000000000000000bfd1681f00e711afa1874b6111be69c4e6b7f3ca": {12277: 1},
    "0x000000000000000000000000841d9604a62543c2c87b7879cc4fc84cec732312": {6929: 2},
    "0x0000000000000000000000002e1bbb7c78a5a534df2442909ab14ce370431e76": {6929: 1},
    "0x000000000000000000000000d7e4b4d56d0f3b646eb8359e932acb9908e72bb3": {6929: 2},
    "0x00000000000000000000000065b8e65213ff91cb922d057c922655c88a125f92": {6929: 1},
    "0x000000000000000000000000e9b0addba12f4ca92589668956b1089d1fdc766e": {6929: 1},
    "0x00000000000000000000000050b294fdb42f152f5dab24cd2b1e357e2e72beb8": {6929: 1},
    "0x0000000000000000000000000832011dbc83892426be0f7b71b4b676f709baf7": {12277: 70},
    "0x000000000000000000000000b0a3b6e4db0f5f9c8f914db1935ad2e6466db0ad": {6929: 1},
    "0x0000000000000000000000001dc5eefe2d20d1ab0db794a4236df147c0f60921": {6929: 1},
    "0x0000000000000000000000006a9758e3cc0403f99f02202691d4cae403690b18": {
        24403: 10,
        12277: 2,
    },
    "0x000000000000000000000000e2dc01d78b90f26a7af877b31c85a8cfc9809b35": {
        24403: 1,
        12277: 1,
    },
    "0x000000000000000000000000a4317ab9d7df7453d8e0853415e04f7e3a78f78f": {24403: 3},
    "0x00000000000000000000000007e9d06db58a77407db95d93fbe1275a46f97463": {6929: 2},
    "0x00000000000000000000000058936684d12df035f6180d6d775508dd34104071": {6929: 1},
    "0x00000000000000000000000079b1c825ae50936a117ffba117d537cb0fd1c2ed": {12277: 1},
    "0x000000000000000000000000db8c3735cdb4ee5588ea085c95e7a24f8e639a4a": {24403: 1},
    "0x000000000000000000000000d05d6dee1e853924b66a548b5793c5f9ef273576": {
        12277: 1,
        6929: 1,
    },
    "0x000000000000000000000000a186727fdaf90cd7d9972572e32c618ce04206f8": {
        24403: 1,
        6929: 1,
        12277: 1,
        30297: 1,
    },
    "0x0000000000000000000000001f00c8221ba028811d4d9e299e6e6caa7a5464c2": {12277: 60},
    "0x00000000000000000000000023aa6a9d5fe63f3281b197386aa009cb56215e22": {30297: 1},
    "0x000000000000000000000000501ac99935d0473d96695bda13a590391b4af413": {30297: 1},
    "0x000000000000000000000000f0a5f95fa8a91ce5483c4d2c4aae771d5eadcfb1": {6929: 5},
    "0x000000000000000000000000e237caaa70218808bc0730c3411be85405fdc835": {6929: 1},
    "0x0000000000000000000000008fa112d9f2944cd1e8ec9ed9c9758f7308abf0ce": {24403: 12},
    "0x000000000000000000000000dc2875911156e71f5e2390c6904b17e4ce082362": {30297: 1},
    "0x000000000000000000000000789989be34af9b69300cc2d0e27e0e7683891e71": {30297: 1},
    "0x000000000000000000000000d46f7c5b3661bb2e4a81cd1eeef7fa8acff43491": {30297: 1},
    "0x0000000000000000000000009f0004e85ab1a65d569cbd9a59a46ef0c84cf470": {6929: 1},
    "0x000000000000000000000000fa380eb338c19c24aa1e07303aeef9f33a2bb59c": {
        6895: 1,
        6929: 9,
        24403: 1,
        30297: 1,
        12277: 1,
        77235: 1,
        88849: 1,
        112043: 1,
    },
    "0x00000000000000000000000067d703b27866043194009a322237e0f623b67c85": {6929: 797},
    "0x000000000000000000000000a628114d249ff3de888c9076a2ce370175e50617": {6929: 514},
    "0x000000000000000000000000a7014054edd2d5a12fce0b7fdb2ea7c1b6b4aa13": {6929: 23},
    "0x0000000000000000000000007dc6bbb117940aa59496af995993133685b07532": {6929: 100},
    "0x000000000000000000000000c063be7510af8d2a9dbfcb8325dc9a8991de7049": {6929: 1},
    "0x0000000000000000000000006d83edcb956bdcf9dddb9d00da7d2728c8bb97e5": {
        6929: 100,
        88849: 5,
        24403: 3,
        30297: 5,
        12277: 3,
        112043: 3,
    },
    "0x00000000000000000000000085d10d3c539bc4bbe13fd1631fa09d14e742eb71": {12277: 1},
    "0x000000000000000000000000eae448e1537aa3c6b2a37e2ab26bba7ca2de9436": {30297: 1},
    "0x000000000000000000000000396078f77c3c9fbdeceae9461166890b323b1361": {77235: 70},
    "0x0000000000000000000000007b9635fa169302e24b999bfac0e9fd30a56ef3cd": {88849: 10},
    "0x000000000000000000000000622f53d0ed07a0cc7ccb85237ba7d722a4c15784": {88849: 1},
    "0x000000000000000000000000d7a9f29f765b1b199f2f0237d1d8690fd470f0aa": {
        6929: 2,
        24403: 3,
    },
    "0x00000000000000000000000020c589ec9258a17b38786673070381de2e93e54f": {30297: 1},
    "0x000000000000000000000000f628c256d1b62727f6bd1968516f2308e4edbdeb": {24403: 1},
    "0x00000000000000000000000015bfc127f8fcf68dfd08da55d2f0c6f404a58b85": {6929: 11},
    "0x000000000000000000000000726cdc837384a7deb8bbea64beba2e7b4d7346c0": {6929: 578},
    "0x00000000000000000000000066dc835a7a14fddbf77ee2dc498ba69df519db7a": {6929: 70},
    "0x000000000000000000000000b89de923dac98daf63948c924bf2bb28ad42a129": {6929: 1},
    "0x00000000000000000000000052fcda4f61108b4e918baba6b671e61c774eaead": {6929: 1},
    "0x0000000000000000000000003a4beacf54c04ea8fa488a2126d6ce1ff022f8cf": {112043: 1},
    "0x000000000000000000000000c3695d7159db8a954a90bac7de28156fab8ccfca": {6929: 1},
    "0x000000000000000000000000485c04fe8357006867e0a7a9ecb941f2b7947d10": {6929: 2},
    "0x0000000000000000000000004826ce4f16a8180915bce7a831726c7ca81e8b24": {6929: 4},
    "0x0000000000000000000000003062dadfc72e65f97912bd73d6cd4ebe93682ff0": {6929: 2},
    "0x000000000000000000000000a9ccc49a14b707b9ec062ad83983c41ffc2a2e4e": {6929: 1},
    "0x000000000000000000000000ccc4dcd160e49584ca8657f39d2a2f59ef520aff": {6929: 2},
    "0x0000000000000000000000003bf2e49ccf93a2babec97408c6219d15cd3a2067": {6929: 1},
    "0x0000000000000000000000009a7ce75fbf99f9a9ca8341fee2def8b89014ab89": {112043: 420},
    "0x000000000000000000000000b5df726e490bc6a17ad44ffe32966678b16596e0": {6929: 1},
    "0x0000000000000000000000005e93aa124a8440ba6dda5bb11ff536e48427bc43": {6929: 1},
    "0x000000000000000000000000feca893272575378dcaca822ebc635514f995a21": {112043: 1},
    "0x0000000000000000000000003066bcfb778f803931198e9e53a7236f46f36e0b": {6929: 1},
    "0x00000000000000000000000003788d19589c8dffad45eecc2fe943a4ad17d167": {6929: 5},
    "0x0000000000000000000000001cb06bf6279b87b831610c88bbe9b58ededcbdae": {6929: 4},
    "0x0000000000000000000000002af61d377fedf34cdaa873e977c2a19bcf307034": {6929: 2},
    "0x0000000000000000000000003dcb8273c23296654a4f372e8f8184ab2fc60fb0": {88849: 1},
    "0x0000000000000000000000002961b9764e7356477a98ad4d46c3243e07d8add4": {6929: 2},
    "0x000000000000000000000000158b8e2b57691e9b1b4c5a84498884064624d1e6": {6929: 1},
    "0x000000000000000000000000cdecae1a8a4386208e5d0692fc913e3556e64467": {6929: 1},
    "0x000000000000000000000000ece9d5e60ff8f715f3b5669496b5948f9374ac67": {6929: 2},
    "0x000000000000000000000000217e882c6d7824e3cc54e9bd6b8fe0bccbc7cf97": {6929: 1},
    "0x00000000000000000000000068d1da77629d8479f56c1d12701eec193366064a": {6929: 5},
    "0x000000000000000000000000ba1f553e781f97b301f58165e2387c964737131b": {6929: 2},
    "0x000000000000000000000000196fb3c4a3ccbd0fadc35d8afe6b29a6cec7c2a1": {6929: 2},
    "0x0000000000000000000000009b4916af44b8046428843ed3c8f33063da3bb289": {6929: 2},
    "0x000000000000000000000000b54c1d1bd294999add2ff8a35806528826fe1d90": {6929: 1},
    "0x0000000000000000000000004c2a781248effa597222c4a3042f5ad7f191cd6f": {6929: 1},
    "0x000000000000000000000000a5d07a4eb94751a22cfe5abe845bf4235ae23670": {6929: 1},
    "0x00000000000000000000000030aa5d9eab492dfa937cfadb1af0054739b0bc2c": {6929: 2},
    "0x000000000000000000000000f9822a4b943d1e9122ec73b782fc4a98562b08a3": {6929: 1},
    "0x00000000000000000000000017dbd30e79189af0b61dafba3126f9113a813597": {6929: 1},
    "0x000000000000000000000000968d7431a29fbff6932a84a8edf75bac0441a1ca": {112043: 1},
    "0x0000000000000000000000009fdc1423d0586d1a08aff702aeb124a8c8692891": {6929: 2},
    "0x0000000000000000000000000ef9fd39ff475602121f25346f8a91f9eb444426": {6929: 3},
    "0x00000000000000000000000043f5969b76440779afd03d435c0a3874c0cd22a9": {6929: 3},
}
flow = {}
unhandled_transactions = []
try:
    while True:
        w3, block = get_block(w3, block_number)
        if block is None:
            print(f'End reached at block {block_number}')
            break
        bloom_filter = BloomFilter(int(Web3.toHex(block.logsBloom), 16))
        if contract_bytes in bloom_filter and (transfer_single_topic in bloom_filter or transfer_batch_topic in bloom_filter):
            block_owners = {}
            for tx_hash in block.transactions:
                w3, transaction_owners = handle_transaction(w3, tx_hash.hex(), flow, unhandled_transactions)
                if transaction_owners is not None and len(transaction_owners) > 0:
                    print(f'{transaction_owners} in {tx_hash.hex()}')
                    merge_owners(block_owners, transaction_owners)
            merge_owners(owners, block_owners)
        block_number += 1
except:
    print(f'owners = {owners}')
    print(f'block_number = {block_number}')
    raise
else:
    print(f'owners = {owners}')
