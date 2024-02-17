import numpy as np
import matplotlib.pyplot as plt

# Load the data
vc = np.loadtxt('vc.csv')
sk = np.loadtxt('sk.csv')

# print(vc, sk)
plt.plot(range(10, 16), vc, label='VC')
plt.plot(range(10, 16), sk, label='SK')

for i, txt in enumerate(vc):
    plt.annotate(txt, (i + 10, vc[i]))
for i, txt in enumerate(sk):
    plt.annotate(txt, (i + 10, sk[i]))


with open('vc.csv', 'w'):
    pass
with open('sk.csv', 'w'):
    pass
plt.legend()
plt.xlabel('Number of threads')
plt.ylabel('Average message size (bytes)')
plt.savefig('plot.png')
plt.show()