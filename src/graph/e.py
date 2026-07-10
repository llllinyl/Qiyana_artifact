import numpy as np
import matplotlib.pyplot as plt
from matplotlib import rcParams

# (A AND B AND C) AND NOT (D OR E) AND F AND NOT G OR H
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
baseline2  = np.array([42549.84, 168129.69, 668589.81, 2695273.98]) 
qiyana2   = np.array([1488.03, 6197.18, 27751.98, 107868.67]) 
qiyanawosel2   = np.array([870.62, 3566.55, 14488.94, 57255.46]) 

coeus3 = np.array([0.086, 0.110, 0.129, 0.214]) 
baseline3  = np.array([0.086, 0.110, 0.129, 0.214]) 
qiyana3   = np.array([0.086, 0.110, 0.129, 0.214]) 
qiyanawosel3   = np.array([0.086, 0.110, 0.129, 0.214]) 

coeus4 = np.array([0.086, 0.144, 0.193, 0.408]) 
baseline4  = np.array([0.086, 0.144, 0.193, 0.408]) 
qiyana4   = np.array([0.086, 0.144, 0.193, 0.408]) 
qiyanawosel4   = np.array([0.086, 0.144, 0.193, 0.408]) 

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
p3 = ax2.bar(x_coeus, coeus3, bar_width, bottom=coeus1 + coeus2,
            label='Coeus-CPU3', color=c_coeus, edgecolor='black', linewidth=0.5, hatch='--')
p4 = ax2.bar(x_coeus, coeus4, bar_width, bottom=coeus1 + coeus2 + coeus3,
            label='Coeus-CPU4', color=c_coeus, edgecolor='black', linewidth=0.5, hatch='|||')

p5 = ax2.bar(x_baseline, baseline1, bar_width,
            label='Baseline-CPU1', color=c_baseline, edgecolor='black', linewidth=0.5)
p6 = ax2.bar(x_baseline, baseline2, bar_width, bottom=baseline1,
            label='Baseline-CPU2', color=c_baseline, edgecolor='black', linewidth=0.5, hatch='x')
p7 = ax2.bar(x_baseline, baseline3, bar_width, bottom=baseline1 + baseline2,
            label='Baseline-CPU3', color=c_baseline, edgecolor='black', linewidth=0.5, hatch='--')
p8 = ax2.bar(x_baseline, baseline4, bar_width, bottom=baseline1 + baseline2 + baseline3,
            label='Baseline-CPU4', color=c_baseline, edgecolor='black', linewidth=0.5, hatch='|||')

p9 = ax2.bar(x_qiyana, qiyana1, bar_width,
            label='Qiyana-CPU1', color=c_qiyana, edgecolor='black', linewidth=0.5)
p10 = ax2.bar(x_qiyana, qiyana2, bar_width, bottom=qiyana1,
            label='Qiyana-CPU2', color=c_qiyana, edgecolor='black', linewidth=0.5, hatch='x')
p11 = ax2.bar(x_qiyana, qiyana3, bar_width, bottom=qiyana1 + qiyana2,
            label='Qiyana-CPU3', color=c_qiyana, edgecolor='black', linewidth=0.5, hatch='--')
p12 = ax2.bar(x_qiyana, qiyana4, bar_width, bottom=qiyana1 + qiyana2 + qiyana3,
            label='Qiyana-CPU4', color=c_qiyana, edgecolor='black', linewidth=0.5, hatch='|||')

p13 = ax2.bar(x_qiyanawosel, qiyanawosel1, bar_width,
            label='Qiyana-wosel-CPU1', color=c_qiyanawosel, edgecolor='black', linewidth=0.5)
p14 = ax2.bar(x_qiyanawosel, qiyanawosel2, bar_width, bottom=qiyanawosel1,
            label='Qiyana-wosel-CPU2', color=c_qiyanawosel, edgecolor='black', linewidth=0.5, hatch='x')
p15 = ax2.bar(x_qiyanawosel, qiyanawosel3, bar_width, bottom=qiyanawosel1 + qiyanawosel2,
            label='Qiyana-wosel-CPU3', color=c_qiyanawosel, edgecolor='black', linewidth=0.5, hatch='--')
p16 = ax2.bar(x_qiyanawosel, qiyanawosel4, bar_width, bottom=qiyanawosel1 + qiyanawosel2 + qiyanawosel3,
            label='Qiyana-wosel-CPU4', color=c_qiyanawosel, edgecolor='black', linewidth=0.5, hatch='|||')

for i in range(len(x_pos)):
    values = [
        coeus1[i] + coeus2[i] + coeus3[i] + coeus4[i],
        baseline1[i] + baseline2[i] + baseline3[i] + baseline4[i],
        qiyana1[i] + qiyana2[i] + qiyana3[i] + qiyana4[i],
        qiyanawosel1[i] + qiyanawosel2[i] + qiyanawosel3[i] + qiyanawosel4[i]
    ]
    max_val = values[1]
    
    # total_coeus = coeus1[i] + coeus2[i]
    # ratio_coeus = int(round(max_val / total_coeus)) if total_coeus > 0 else 0
    # offset = 0.1 if i > 0 else 0.075
    # ax2.text(x_coeus[i] - offset, total_coeus * 1.1, f'{ratio_coeus}' + r'$\times$', 
    #          ha='center', va='bottom', fontsize=9, family='Times New Roman',fontweight='bold')
    
    total_qiyana = qiyana1[i] + qiyana2[i]
    ratio_qiyana = max_val / total_qiyana
    ax2.text(x_qiyana[i] + 0.14, total_qiyana * 1.35, f'{ratio_qiyana:.1f}' + r'$\times$', 
             ha='center', va='bottom', fontsize=12, family='Times New Roman',fontweight='bold')
    
    total_qiyanawosel = qiyanawosel1[i] + qiyanawosel2[i]
    ratio_qiyanawosel = max_val / total_qiyanawosel
    ax2.text(x_qiyanawosel[i] + 0.16, total_qiyanawosel * 0.95, f'{ratio_qiyanawosel:.1f}' + r'$\times$', 
             ha='center', va='bottom', fontsize=12, family='Times New Roman',fontweight='bold')

ax2.set_xlim(x_pos[0] - 3*bar_width, x_pos[-1] + 4.3*bar_width)
ax2.set_xticks(x_pos)
ax2.set_xticklabels(x_labels, fontsize=15)

from matplotlib.patches import Patch
legend_elements = [
    Patch(facecolor=c_coeus, edgecolor='black', linewidth=0.4, label='Coeus'),
    Patch(facecolor=c_baseline, edgecolor='black', linewidth=0.4, label='Baseline'),
    Patch(facecolor=c_qiyana, edgecolor='black', linewidth=0.4, label='Qiyana'+ r'$_0$'),
    Patch(facecolor=c_qiyanawosel, edgecolor='black', linewidth=0.4, label='Qiyana'+ r'$_1$'),
]

ax2.legend(handles=legend_elements, ncol=2, loc='upper left', 
           fontsize=11, frameon=True, framealpha=0.95, bbox_to_anchor=(-0.05, 1),
           handlelength=0.8, handletextpad=0.2, borderpad=0.2, labelspacing=0.2)  

ax1.set_xlim(0, 1)
ax1.set_xticks([0])
ax1.set_xticklabels(['0'], fontsize=12)

ax1.set_ylim(1, 5000000)
ax1.set_yscale('log')
ax1.set_yticks([10, 100, 1000, 10000, 100000, 1000000])
ax1.set_yticklabels(['$10$', '$10^2$', '$10^3$', '$10^4$', '$10^5$', '$10^6$'], fontsize=12)

ax1.spines['right'].set_visible(False)
ax2.spines['left'].set_visible(False)

ax2.yaxis.set_visible(False)  
# ax1.tick_params(axis="both", which="major", direction="in", width=1, length=5, labelsize=10)
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

plt.savefig('e.pdf', dpi=600, bbox_inches='tight')
plt.show()