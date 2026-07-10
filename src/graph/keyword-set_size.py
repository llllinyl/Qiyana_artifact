import numpy as np
import matplotlib.pyplot as plt
from matplotlib import rcParams

config = {
    "font.family": 'serif',
    "font.serif": ['Times New Roman', 'SimSun'],
    "mathtext.fontset": 'stix',
    "figure.dpi": 300,
}
plt.rcParams.update(config)

x_positions = np.array([1, 2, 3, 4, 5, 6])
x_labels = ['Baseline', 'Qiyana'+ r'$_0$', 'Qiyana'+ r'$_1$', 'Baseline', 'Qiyana'+ r'$_0$', 'Qiyana'+ r'$_1$']

data_2 = [1308.97+3.78, 924.97, 137.25+3.78, 1291.84+3.78, 1159.16, 360.22+3.78]
data_4 = [2571.16+3.78, 927.38, 137.49+3.78, 2510.75+3.78, 1155.39, 365.66+3.78]
data_8 = [5115.87+3.78, 922.86, 136.81+3.78, 5069.48+3.78, 1160.65, 362.11+3.78]
data_16 = [10497.64+3.78, 925.02, 134.97+3.78, 10450.67+3.78, 1157.88, 364.23+3.78]

myblue = '#B3CEFF'
myyellow = '#FFEBB7'
myred = '#E4A9A4'
mypurple = '#B99EC6'
black = '#000000'

fig, ax1 = plt.subplots(figsize=(5, 2)) 

ax1.set_ylabel('Latency (s)', fontsize=11, family='Times New Roman', labelpad=0)
ax1.set_ylim(50, 50000)
ax1.set_xlim(0.5, 6.5)
ax1.set_yscale('log')
ax1.set_yticks([100, 1000, 10000])
ax1.set_yticklabels(['$10^2$', '$10^3$', '$10^4$'], fontsize=9)

ax1.set_xticks(x_positions)
ax1.set_xticklabels(x_labels, ha='center', fontsize=10, family='Times New Roman')

bar_width = 0.125
data_sets = [data_2, data_4, data_8, data_16]
colors = [myblue, myyellow, myred, mypurple]
labels = ['2', '4', '8', '16']
shifts = [-1.875*bar_width, -0.625*bar_width, 0.625*bar_width, 1.875*bar_width]

for i, (data, color, label, shift) in enumerate(zip(data_sets, colors, labels, shifts)):
    x_shifted = x_positions + shift
    bars = ax1.bar(x_shifted, data, width=bar_width, color=color, 
                   edgecolor='black', linewidth=0.5, label=label)

ax1.axvline(x=3.5, color='red', linestyle='-.', linewidth=1.5)

ax1.text(2, 23500, 'A AND B', color='red', fontsize=10, ha='center', 
         family='Times New Roman', fontweight='bold', 
         bbox=dict(boxstyle='round,pad=0.2', facecolor='white', alpha=0.8))
ax1.text(5, 23500, 'A OR B', color='blue', fontsize=10, ha='center', 
         family='Times New Roman', fontweight='bold', 
         bbox=dict(boxstyle='round,pad=0.2', facecolor='white', alpha=0.8))

y_min1 = data_2[0]
ax1.hlines(y=y_min1, xmin=0.75, xmax=3.25, 
           colors='purple', linestyles='--', linewidth=0.75, alpha=0.7)

y_qiyana0_and_max = data_16[1]
y_qiyana1_and_max = data_8[2]
ratio1 = y_min1 / y_qiyana0_and_max
ratio2 = y_min1 / y_qiyana1_and_max

ax1.annotate(r'$\geq$' + f'{ratio1:.1f}x', xy=(2, y_min1 * 1.25), 
             ha='center', va='bottom', fontsize=9,
             fontweight='bold', family='Times New Roman',
             bbox=dict(boxstyle='round,pad=0.2', facecolor='white', alpha=0.8))

ax1.annotate(r'$\geq$' + f'{ratio2:.1f}x', xy=(3, y_min1 * 1.25), 
             ha='center', va='bottom', fontsize=9,
             fontweight='bold', family='Times New Roman',
             bbox=dict(boxstyle='round,pad=0.2', facecolor='white', alpha=0.8))

y_min2 = data_2[3]
ax1.hlines(y=y_min2, xmin=3.75, xmax=6.25, 
           colors='orange', linestyles='--', linewidth=0.75, alpha=0.7)

y_qiyana0_or_max = data_8[4]
y_qiyana1_or_max = data_16[5]
ratio3 = y_min2 / y_qiyana0_or_max
ratio4 = y_min2 / y_qiyana1_or_max

ax1.annotate(r'$\geq$' + f'{ratio3:.1f}x', xy=(5, y_min2 * 1.25), 
             ha='center', va='bottom', fontsize=9,
             fontweight='bold', family='Times New Roman',
             bbox=dict(boxstyle='round,pad=0.2', facecolor='white', alpha=0.8))

ax1.annotate(r'$\geq$' + f'{ratio4:.1f}x', xy=(6, y_min2 * 1.25), 
             ha='center', va='bottom', fontsize=9,
             fontweight='bold', family='Times New Roman',
             bbox=dict(boxstyle='round,pad=0.2', facecolor='white', alpha=0.8))

y_max1 = data_16[0]
ax1.hlines(y=y_max1, xmin=0.75, xmax=3.25, 
           colors='purple', linestyles='--', linewidth=0.75, alpha=0.7)

y_qiyana0_and_max = data_8[1]
y_qiyana1_and_max = data_16[2]
ratio1 = y_max1 / y_qiyana0_and_max
ratio2 = y_max1 / y_qiyana1_and_max

ax1.annotate(r'$\leq$' + f'{ratio1:.1f}x', xy=(2, y_max1 * 0.475), 
             ha='center', va='bottom', fontsize=9,
             fontweight='bold', family='Times New Roman',
             bbox=dict(boxstyle='round,pad=0.2', facecolor='white', alpha=0.8))

ax1.annotate(r'$\leq$' + f'{ratio2:.1f}x', xy=(3, y_max1 * 0.475), 
             ha='center', va='bottom', fontsize=9,
             fontweight='bold', family='Times New Roman',
             bbox=dict(boxstyle='round,pad=0.2', facecolor='white', alpha=0.8))

y_max2 = data_16[3]
ax1.hlines(y=y_max2, xmin=3.75, xmax=6.25, 
           colors='orange', linestyles='--', linewidth=0.75, alpha=0.7)

y_qiyana0_or_max = data_4[4]
y_qiyana1_or_max = data_2[5]
ratio3 = y_max2 / y_qiyana0_or_max
ratio4 = y_max2 / y_qiyana1_or_max

ax1.annotate(r'$\leq$' + f'{ratio3:.1f}x', xy=(5, y_max2 * 0.475), 
             ha='center', va='bottom', fontsize=9,
             fontweight='bold', family='Times New Roman',
             bbox=dict(boxstyle='round,pad=0.2', facecolor='white', alpha=0.8))

ax1.annotate(r'$\leq$' + f'{ratio4:.1f}x', xy=(6, y_max2 * 0.475), 
             ha='center', va='bottom', fontsize=9,
             fontweight='bold', family='Times New Roman',
             bbox=dict(boxstyle='round,pad=0.2', facecolor='white', alpha=0.8))

handles1, labels1 = ax1.get_legend_handles_labels()
ax1.legend(handles1, labels1, ncol=2, loc='upper right', 
           bbox_to_anchor=(1.01, 1.02), fontsize=7.5, frameon=True, 
           framealpha=0.7, columnspacing=0.9, handlelength=1,
           handletextpad=0.3, borderpad=0.2) 

# ax1.tick_params(axis="both", which="major", direction="in", width=0.8, length=4, labelsize=10)

plt.tight_layout()
plt.savefig('lkss.pdf', dpi=600, bbox_inches='tight')
plt.show()