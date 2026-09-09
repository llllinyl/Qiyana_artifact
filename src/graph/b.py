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
baselineh1  = np.array([2.04, 2.19, 3.04, 6.79]) 
qiyana01   = np.array([0, 0, 0, 0]) 
qiyana11   = np.array([2.04, 2.19, 3.04, 6.79]) 

coeus2 = np.array([0, 0, 0, 0]) 
baseline2  = np.array([10276.51, 42712.12, 169005.37, 675357.01]) 
baselineh2  = np.array([4167.48, 16090.84, 64297.18, 254077.34]) 
qiyana02   = np.array([946.29, 3914.79, 18395.25, 73418.69]) 
qiyana12   = np.array([300.47, 1172.35, 4826.75, 19607.56]) 

coeus3 = np.array([0.086, 0.110, 0.129, 0.214]) 
baseline3  = np.array([0.086, 0.110, 0.129, 0.214]) 
baselineh3  = np.array([0.086, 0.110, 0.129, 0.214]) 
qiyana03   = np.array([0.086, 0.110, 0.129, 0.214]) 
qiyana13   = np.array([0.086, 0.110, 0.129, 0.214]) 

coeus4 = np.array([0.086, 0.144, 0.193, 0.408]) 
baseline4  = np.array([0.086, 0.144, 0.193, 0.408]) 
baselineh4  = np.array([0.086, 0.144, 0.193, 0.408]) 
qiyana04   = np.array([0.086, 0.144, 0.193, 0.408]) 
qiyana14   = np.array([0.086, 0.144, 0.193, 0.408]) 

fig, (ax1, ax2) = plt.subplots(1, 2, sharey=True, figsize=(5, 3),
                               gridspec_kw={'width_ratios': [1, 35], 'wspace': 0.03})

c_qiyana1 = '#B3CEFF'
c_qiyana0 = '#FFEBB7'
c_baselineh = '#81B095'
c_baseline = '#E4A9A4'
c_coeus = '#B99EC6'

bar_width = 0.12

x_coeus = x_pos - 2 * bar_width
x_baseline = x_pos - bar_width
x_baselineh = x_pos
x_qiyana0 = x_pos + bar_width
x_qiyana1 = x_pos + 2 * bar_width


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

p9 = ax2.bar(x_baselineh, baselineh1, bar_width,
            label='Baselineh-CPU1', color=c_baselineh, edgecolor='black', linewidth=0.5)
p10 = ax2.bar(x_baselineh, baselineh2, bar_width, bottom=baselineh1,
            label='Baselineh-CPU2', color=c_baselineh, edgecolor='black', linewidth=0.5, hatch='x')
p11 = ax2.bar(x_baselineh, baselineh3, bar_width, bottom=baselineh1 + baselineh2,
            label='Baselineh-CPU3', color=c_baselineh, edgecolor='black', linewidth=0.5, hatch='--')
p12 = ax2.bar(x_baselineh, baselineh4, bar_width, bottom=baselineh1 + baselineh2 + baselineh3,
            label='Baselineh-CPU4', color=c_baselineh, edgecolor='black', linewidth=0.5, hatch='|||')

p13 = ax2.bar(x_qiyana0, qiyana01, bar_width,
            label='Qiyana0-CPU1', color=c_qiyana0, edgecolor='black', linewidth=0.5)
p14 = ax2.bar(x_qiyana0, qiyana02, bar_width, bottom=qiyana01,
            label='Qiyana0-CPU2', color=c_qiyana0, edgecolor='black', linewidth=0.5, hatch='x')
p15 = ax2.bar(x_qiyana0, qiyana03, bar_width, bottom=qiyana01 + qiyana02,
            label='Qiyana0-CPU3', color=c_qiyana0, edgecolor='black', linewidth=0.5, hatch='--')
p16 = ax2.bar(x_qiyana0, qiyana04, bar_width, bottom=qiyana01 + qiyana02 + qiyana03,
            label='Qiyana0-CPU4', color=c_qiyana0, edgecolor='black', linewidth=0.5, hatch='|||')

p17 = ax2.bar(x_qiyana1, qiyana11, bar_width,
            label='Qiyana1-CPU1', color=c_qiyana1, edgecolor='black', linewidth=0.5)
p18 = ax2.bar(x_qiyana1, qiyana12, bar_width, bottom=qiyana11,
            label='Qiyana1-CPU2', color=c_qiyana1, edgecolor='black', linewidth=0.5, hatch='x')
p10 = ax2.bar(x_qiyana1, qiyana13, bar_width, bottom=qiyana11 + qiyana12,
            label='Qiyana1-CPU3', color=c_qiyana1, edgecolor='black', linewidth=0.5, hatch='--')
p20 = ax2.bar(x_qiyana1, qiyana14, bar_width, bottom=qiyana11 + qiyana12 + qiyana13,
            label='Qiyana1-CPU4', color=c_qiyana1, edgecolor='black', linewidth=0.5, hatch='|||')

for i in range(len(x_pos)):
    values = [
        coeus1[i] + coeus2[i] + coeus3[i] + coeus4[i],
        baseline1[i] + baseline2[i] + baseline3[i] + baseline4[i],
        baselineh1[i] + baselineh2[i] + baselineh3[i] + baselineh4[i],
        qiyana01[i] + qiyana02[i] + qiyana03[i] + qiyana04[i],
        qiyana11[i] + qiyana12[i] + qiyana13[i] + qiyana14[i]
    ]
    max_val = values[1]

    ax2.text(x_baselineh[i], values[1], f'{1.0:.1f}' + r'$\times$', 
             ha='center', va='bottom', fontsize=11, family='Times New Roman',fontweight='bold')
    
    ratio_baselineh0 = max_val / values[2]
    ax2.text(x_baselineh[i] + 0.10, values[2], f'{ratio_baselineh0:.1f}' + r'$\times$', 
             ha='center', va='bottom', fontsize=11, family='Times New Roman',fontweight='bold')
    
    ratio_qiyana0 = max_val / values[3]
    ax2.text(x_qiyana0[i] + 0.14, values[3], f'{ratio_qiyana0:.1f}' + r'$\times$', 
             ha='center', va='bottom', fontsize=11, family='Times New Roman',fontweight='bold')
    
    ratio_qiyana1 = max_val / values[4]
    ax2.text(x_qiyana1[i] + 0.14, values[4], f'{ratio_qiyana1:.1f}' + r'$\times$', 
             ha='center', va='bottom', fontsize=11, family='Times New Roman',fontweight='bold')

ax2.set_xlim(x_pos[0] - 3*bar_width, x_pos[-1] + 4.3*bar_width)
ax2.set_xticks(x_pos)
ax2.set_xticklabels(x_labels, fontsize=15)

from matplotlib.patches import Patch
legend_elements = [
    Patch(facecolor=c_coeus, edgecolor='black', linewidth=0.4, label='Coeus'),
    Patch(facecolor=c_baseline, edgecolor='black', linewidth=0.4, label='Baseline'),
    Patch(facecolor=c_baselineh, edgecolor='black', linewidth=0.4, label='BaselineH'),
    Patch(facecolor=c_qiyana0, edgecolor='black', linewidth=0.4, label='Qiyana'+ r'$_0$'),
    Patch(facecolor=c_qiyana1, edgecolor='black', linewidth=0.4, label='Qiyana'+ r'$_1$'),
]

ax2.legend(handles=legend_elements, ncol=3, loc='upper left', fontsize=10, 
            frameon=True, framealpha=0.95, bbox_to_anchor=(-0.05, 1), columnspacing=0.2,
            handlelength=0.8, handletextpad=0.1, borderpad=0.1, labelspacing=0.1)  

ax1.set_xlim(0, 1)
ax1.set_xticks([0])
ax1.set_xticklabels(['0'], fontsize=12)

ax1.set_ylim(1, 2500000)
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