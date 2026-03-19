import numpy as np
import matplotlib.pyplot as plt
from matplotlib import rcParams

# A OR B
config = {
    "font.family": 'serif',
    "font.serif": ['Times New Roman', 'SimSun'],
    "mathtext.fontset": 'stix',
    "figure.dpi": 300,
}
plt.rcParams.update(config)

x_full = np.array([16, 64, 256, 1024])
x_labels = ['16K', '64K', '256K', '1M']
x_pos = np.arange(len(x_full))

coeus1 = np.array([2.04, 2.19, 3.04, 6.79]) 
baseline1  = np.array([2.04, 2.19, 3.04, 6.79]) 
qiyana1   = np.array([0, 0, 0, 0]) 
qiyanawosel1   = np.array([2.04, 2.19, 3.04, 6.79]) 

coeus2 = np.array([0, 0, 0, 0]) 
baseline2  = np.array([10276.51, 42712.12, 169005.37, 675357.01]) 
qiyana2   = np.array([946.29, 3914.79, 18395.25, 73418.69]) 
qiyanawosel2   = np.array([300.47, 1172.35, 4826.75, 19607.56]) 

fig, (ax1, ax2) = plt.subplots(1, 2, sharey=True, figsize=(5, 3),
                               gridspec_kw={'width_ratios': [1, 35], 'wspace': 0.03})

c_qiyanawosel = '#B3CEFF'
c_qiyana = '#FFEBB7'
c_baseline = '#E4A9A4'
c_coeus = '#B99EC6'

bar_width = 0.15

x_coeus = x_pos - 1.875*bar_width
x_baseline = x_pos - 0.625*bar_width
x_qiyana = x_pos + 0.625*bar_width
x_qiyanawosel = x_pos + 1.875*bar_width


p1 = ax2.bar(x_coeus, coeus1, bar_width, 
            label='Coeus-CPU1', color=c_coeus, edgecolor='black', linewidth=0.5)
p2 = ax2.bar(x_coeus, coeus2, bar_width, bottom=coeus1,
            label='Coeus-CPU2', color=c_coeus, edgecolor='black', linewidth=0.5, hatch='x')

p3 = ax2.bar(x_baseline, baseline1, bar_width,
            label='Baseline-CPU1', color=c_baseline, edgecolor='black', linewidth=0.5)
p4 = ax2.bar(x_baseline, baseline2, bar_width, bottom=baseline1,
            label='Baseline-CPU2', color=c_baseline, edgecolor='black', linewidth=0.5, hatch='x')

p5 = ax2.bar(x_qiyana, qiyana1, bar_width,
            label='Qiyana-CPU1', color=c_qiyana, edgecolor='black', linewidth=0.5)
p6 = ax2.bar(x_qiyana, qiyana2, bar_width, bottom=qiyana1,
            label='Qiyana-CPU2', color=c_qiyana, edgecolor='black', linewidth=0.5, hatch='x')

p7 = ax2.bar(x_qiyanawosel, qiyanawosel1, bar_width,
            label='Qiyana-wosel-CPU1', color=c_qiyanawosel, edgecolor='black', linewidth=0.5)
p8 = ax2.bar(x_qiyanawosel, qiyanawosel2, bar_width, bottom=qiyanawosel1,
            label='Qiyana-wosel-CPU2', color=c_qiyanawosel, edgecolor='black', linewidth=0.5, hatch='x')

for i in range(len(x_pos)):
    values = [
        coeus1[i] + coeus2[i],
        baseline1[i] + baseline2[i],
        qiyana1[i] + qiyana2[i],
        qiyanawosel1[i] + qiyanawosel2[i]
    ]
    max_val = values[1]
    
    # total_coeus = coeus1[i] + coeus2[i]
    # ratio_coeus = int(round(max_val / total_coeus)) if total_coeus > 0 else 0
    # offset = 0.1 if i > 0 else 0.075
    # ax2.text(x_coeus[i] - offset, total_coeus * 1.1, f'{ratio_coeus}' + r'$\times$', 
    #          ha='center', va='bottom', fontsize=9, family='Times New Roman',fontweight='bold')
    
    total_qiyana = qiyana1[i] + qiyana2[i]
    ratio_qiyana = max_val / total_qiyana
    ax2.text(x_qiyana[i] + 0.14, total_qiyana * 1.1, f'{ratio_qiyana:.1f}' + r'$\times$', 
             ha='center', va='bottom', fontsize=12, family='Times New Roman',fontweight='bold')
    
    total_qiyanawosel = qiyanawosel1[i] + qiyanawosel2[i]
    ratio_qiyanawosel = max_val / total_qiyanawosel
    ax2.text(x_qiyanawosel[i] + 0.16, total_qiyanawosel * 1.1, f'{ratio_qiyanawosel:.1f}' + r'$\times$', 
             ha='center', va='bottom', fontsize=12, family='Times New Roman',fontweight='bold')

ax2.set_xlim(x_pos[0] - 3*bar_width, x_pos[-1] + 4.3*bar_width)
ax2.set_xticks(x_pos)
ax2.set_xticklabels(x_labels, fontsize=15)

from matplotlib.patches import Patch
legend_elements = [
    Patch(facecolor=c_coeus, edgecolor='black', linewidth=0.4, label='Coeus'),
    Patch(facecolor=c_baseline, edgecolor='black', linewidth=0.4, label='Baseline'),
    Patch(facecolor=c_qiyana, edgecolor='black', linewidth=0.4, label='Qiyana'),
    Patch(facecolor=c_qiyanawosel, edgecolor='black', linewidth=0.4, label='Qiyana-wosel'),
]

ax2.legend(handles=legend_elements, ncol=2, loc='upper left', 
           fontsize=11, frameon=True, framealpha=0.95, bbox_to_anchor=(-0.05, 1),
           handlelength=0.8, handletextpad=0.2, borderpad=0.2, labelspacing=0.2)  

ax1.set_xlim(0, 1)
ax1.set_xticks([0])
ax1.set_xticklabels(['0'], fontsize=12)

ax1.set_ylim(1, 1000000)
ax1.set_yscale('log')
ax1.set_yticks([10, 100, 1000, 10000, 100000])
ax1.set_yticklabels(['$10$', '$10^2$', '$10^3$', '$10^4$', '$10^5$'], fontsize=12)

ax1.spines['right'].set_visible(False)
ax2.spines['left'].set_visible(False)

ax2.yaxis.set_visible(False)  
# ax1.tick_params(axis="both", which="major", direction="in", width=1, length=5, labelsize=15)
# ax2.tick_params(axis="x", which="major", direction="in", width=1, length=0, labelsize=15)

d = .02
kwargs = dict(transform=ax1.transAxes, color='k', clip_on=False)
ax1.plot((1 - d, 1 + d), (-d, +d), **kwargs)
ax1.plot((1 - d, 1 + d), (1 - d, 1 + d), **kwargs)

kwargs.update(transform=ax2.transAxes)
ax2.plot((-d * 0.03, +d * 0.03), (-d, +d), **kwargs)
ax2.plot((-d * 0.03, +d * 0.03), (1 - d, 1 + d), **kwargs)

fig.text(0.52, 0.02, "Dataset Size " + r'$(|\mathcal{D}|)$', ha='center', fontsize=15, family='Times New Roman')
ax1.set_ylabel('Latency (s)', fontsize=15, family='Times New Roman', labelpad=0)
plt.subplots_adjust(bottom=0.16)
plt.tight_layout()

plt.savefig('b.pdf', dpi=600, bbox_inches='tight')
plt.show()