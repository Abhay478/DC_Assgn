import os
import math
import sys

with open('inp-params.txt') as f:
    s = f.read().split(' ')
    n = int(s[0])
    k = int(s[1])
    print(n, k)
    with open('ips.txt', 'w') as f:
        for i in range(n):
            print(f'{int(i // math.sqrt(n))} {int(i % math.sqrt(n))} 0.0.0.0 {8080 + i}', file=f)

    for i in range(n):
        print('.')
        # subprocess.c run(['cargo', 'r', '--release', '-q', '--bin', 'q2', '--', str(i)])
        # multiprocessing.Process(target=subprocess.run, args=(['cargo', 'r', '--release', '-q', '--bin', 'q2', '--', str(i)],)).start()
        if sys.argv[1] == '1':
            os.system(f'cargo r -q --bin q2 -- {i} &')
        else:
            cmd = f'cargo r -q --bin q1 -- {int(i // math.sqrt(n))} {int(i % math.sqrt(n))} &'
            print(cmd)
            os.system(cmd)
