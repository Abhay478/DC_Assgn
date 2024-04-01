import sys

with open('inp-params.txt') as f:
    s = f.read().split(' ')
    n = int(s[0])
    k = int(s[1])
    q = 0
    p = 'rc' if sys.argv[1] == '1' else 'maekawa'
    for i in range(n):
        r = str(i) if sys.argv[1] == '1' else f'{int(i // n)}_{int(i % n)}'
        with open(f'log/{p}/out_{r}.log') as f:
            s = f.read().split(' ')
            q += int(s[0])
    # q /= n
    print(q)