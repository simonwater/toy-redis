这个项目是针对["Build Your Own Redis"](https://codecrafters.io/challenges/redis)挑战的完整解决方案，通过rust语言实现了整个挑战的所有需求。

*（截止2026-11-11，后续官方可能会增加新的挑战关卡而toy-redis可能未实现新关卡）*

# 运行项目
1. 确保本地cargo版本不低于1.96
2. 在项目根目录运行命令：`cargo run`，默认在6379端口启动 toy-redis 服务端
3. 安装redis官方命令行客户端 [redis-cli](https://redis.io/docs/latest/operate/oss_and_stack/install/install-stack/install-redis-cli/)
4. 在命令行终端运行redis-cli：
```shell
redis-cli -h 127.0.0.1 -p 6379
```
5. 启动cli后执行redis命令：
```shell
127.0.0.1:6379> PING
PONG
```

# 通过 Codecrafters 官方测试套件测试toy-redis
Codecrafters 官方挑战的所有测试用例，只有在升级会员资格以后才能在线运行评测代码。幸运的是， codecrafters 已经在 github 上开源了所有测试套件，没有会员资格的用户可以下载测试套件源码，通过本地编译然后离线运行测试用例来验证自己的代码。这里介绍整个过程。
## 编译官方测试套件
1. 由于官方的测试套件是由go语言编写的，请先确保本地已经安装好go
2. 下载[Build Your Own Redis](https://codecrafters.io/challenges/redis)项目的测试套件源码：[redis-tester](https://github.com/codecrafters-io/redis-tester)
3. 在 redis-tester 源码根目录运行编译命令：
```shell
go build -o dist/tester.out ./cmd/tester
```
4. 将编译得到的可执行文件 `dist/tester.out` 拷贝到 toy-redis 项目根目录，覆盖原来的 `tester.out` 文件

## 运行测试套件
[Build Your Own Redis](https://codecrafters.io/challenges/redis) 挑战的每一个关卡都有一个简写的别名 slug，比如第一个关卡 [Bind to a port #jm1](https://app.codecrafters.io/courses/redis/stages/jm1)，'#'后面跟着的 `jm1` 就是关卡 slug。toy-redis 测试脚本支持以下几种运行模式，注意命令中关卡 slug 必须为小写： 
1. 运行某个关卡(比如: jm1)的测试用例，在根目录执行： 
```shell
./test.sh jm1
```
2. 运行某个关卡及该关卡之前的所有关卡：
```shell
./test.sh yz1 -p
```
3. 运行所有关卡的测试用例：
```shell
./test.sh
```

# 实现的功能
toy-redis 实现的是标准 redis 的一个子集，功能涵盖了[Build Your Own Redis](https://codecrafters.io/challenges/redis) 挑战项目截止 **2026-11-11** 的所有挑战关卡，关卡分组内容如下：
- [x] Base
- [x] Lists
- [x] Streams
- [x] Transactions
- [x] Optimistic locking
- [x] Replication
- [x] RDB Persistence
- [x] AOF Persistence
- [x] Pub/Sub
- [x] Sorted Sets
- [x] Bitmaps
- [x] Geospatial Commands
- [x] Authentication