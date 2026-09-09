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

x_labels = ['1-AND', '2-AND', '3-AND', 
            '1-OR', '2-OR', '3-OR']
x_positions = np.array([1, 2, 3, 4, 5, 6])

baseline_and = [10497.64+3.78, 15778.43+3.78, 20852.99+3.78]
baselineh_and = [3982.95+3.78, 5958.26+3.78, 8041.30+3.78]
qiyana0_and = [927.14, 924.51, 927.84]
qiyana1_and = [140.84+3.78, 139.95+3.78, 140.59+3.78]

baseline_or = [10450.67+3.78, 15401.97+3.78, 20388.71+3.78]
baselineh_or = [3951.68+3.78, 6029.22+3.78, 7966.73+3.78]
qiyana0_or = [1165.73, 1413.73, 1621.64]
qiyana1_or = [365.11+3.78, 592.75+3.78, 846.01+3.78]

comm_x = [1, 2, 3, 4, 5, 6]
comm_baseline = [262.77, 264.78, 266.79, 262.77, 264.78, 266.79]
comm_baselineh = [260.13, 260.82, 261.51, 260.13, 260.82, 261.51]
comm_qiyana0 = [23.32, 23.32, 23.32, 39.89, 56.46, 73.03]
comm_qiyana1 = [150.32, 150.32, 150.32, 166.89, 183.46, 200.03]

c_baseline = '#d62728'
c_baselineh = '#9467BD'
c_qiyana0 = '#1f77b4'
c_qiyana1 = '#ff7f0e'

myred = '#E4A9A4'
mypurple = '#D8C7E8'
myblue = '#B3CEFF'
myyellow = '#FFEBB7'

fig, ax1 = plt.subplots(figsize=(5, 2))

ax1.set_facecolor('none') 
ax1.set_ylabel('Latency (s)', fontsize=11, family='Times New Roman', labelpad=0)
ax1.set_ylim(50, 500000)
ax1.set_xlim(0.5, 6.5)
ax1.set_yscale('log')
ax1.set_yticks([100, 1000, 10000, 100000])
ax1.set_yticklabels(['$10^2$', '$10^3$', '$10^4$', '$10^5$'], fontsize=9)

ax1.set_xticks(x_positions)
ax1.set_xticklabels(x_labels, ha='center', fontsize=11, family='Times New Roman')

x_and = x_positions[:4]

ax1.plot(x_positions[:3], baseline_and, marker='o', markersize=4, 
         color=c_baseline, linewidth=1.5, label='Baseline (AND)', zorder=10)

ax1.plot(x_positions[:3], baselineh_and,marker='s', markersize=4,
    color=c_baselineh, linewidth=1.5,label='BaselineH (AND)', zorder=10)

ax1.plot(x_positions[:3], qiyana0_and, marker='x', markersize=4, 
         color=c_qiyana0, linewidth=1.5, label='Qiyana'+ r'$_0$'+ ' (AND)', zorder=10)

ax1.plot(x_positions[:3], qiyana1_and, marker='*', markersize=4, 
         color=c_qiyana1, linewidth=1.5, label='Qiyana'+ r'$_1$' +' (AND)', zorder=10)

ax1.plot(x_positions[3:], baseline_or, marker='o', markersize=4, 
         color=c_baseline, linewidth=1.5, linestyle='--', 
         label='Baseline (OR)', zorder=10)

ax1.plot(x_positions[3:], baselineh_or,marker='s', markersize=4,
        color=c_baselineh, linewidth=1.5,linestyle='--',
        label='BaselineH (OR)', zorder=10)

ax1.plot(x_positions[3:], qiyana0_or, marker='x', markersize=4, 
         color=c_qiyana0, linewidth=1.5, linestyle='--', 
         label='Qiyana'+ r'$_0$'+ ' (OR)', zorder=10)

ax1.plot(x_positions[3:], qiyana1_or, marker='*', markersize=4, 
         color=c_qiyana1, linewidth=1.5, linestyle='--', 
         label='Qiyana'+ r'$_1$' +' (OR)', zorder=10)

def add_ratio_arrow(ax, x_pos, y_low, y_high, offset_x=0):
    ratio = y_high / y_low
    
    x_arrow = x_pos + offset_x
    
    ax.annotate('', xy=(x_arrow, y_high), xytext=(x_arrow, y_low),
                arrowprops=dict(arrowstyle='<->', color='black', lw=0.8), zorder=15)
    
    mid_y = np.sqrt(y_low * y_high)
    if offset_x < 0:
        ax.annotate(f'{ratio:.1f}x', xy=(x_arrow + offset_x, mid_y), 
                    ha='right', va='center', fontsize=9,
                    fontweight='bold', family='Times New Roman',
                    bbox=dict(boxstyle='round,pad=0.1', facecolor='white', alpha=0.7, edgecolor='none'),
                    zorder=20)
    else: 
        ax.annotate(f'{ratio:.1f}x', xy=(x_arrow + offset_x, mid_y), 
                    ha='left', va='center', fontsize=9,
                    fontweight='bold', family='Times New Roman',
                    bbox=dict(boxstyle='round,pad=0.1', facecolor='white', alpha=0.7, edgecolor='none'),
                    zorder=20)

add_ratio_arrow(ax1, 1, qiyana0_and[0], baselineh_and[0], offset_x=-0.04)
add_ratio_arrow(ax1, 1, qiyana1_and[0], baselineh_and[0], offset_x=0.04)

add_ratio_arrow(ax1, 3, qiyana0_and[2], baselineh_and[2], offset_x=-0.04)
add_ratio_arrow(ax1, 3, qiyana1_and[2], baselineh_and[2], offset_x=0.04)

add_ratio_arrow(ax1, 4, qiyana0_or[0], baselineh_or[0], offset_x=-0.04)
add_ratio_arrow(ax1, 4, qiyana1_or[0], baselineh_or[0], offset_x=0.04)

add_ratio_arrow(ax1, 6, qiyana0_or[2], baselineh_or[2], offset_x=-0.04)
add_ratio_arrow(ax1, 6, qiyana1_or[2], baselineh_or[2], offset_x=0.04)

ax1.axvline(x=3.5, color='red', linestyle='-.', linewidth=1.5)

ax1.text(3.4, 150000, 'AND Queries', color='red', fontsize=10, ha='right', 
         family='Times New Roman', fontweight='bold', 
         bbox=dict(boxstyle='round,pad=0.2', facecolor='white', alpha=0.8))
ax1.text(3.6, 150000, 'OR Queries', color='blue', fontsize=10, ha='left', 
         family='Times New Roman', fontweight='bold', 
         bbox=dict(boxstyle='round,pad=0.2', facecolor='white', alpha=0.8))

ax2 = ax1.twinx()
ax2.set_zorder(ax1.get_zorder() - 1)
ax2.set_ylabel('Communication (MB)', fontsize=11, family='Times New Roman', labelpad=0)
ax2.set_ylim(5, 1500)
ax2.set_yscale('log')
ax2.set_yticks([10, 50, 200])
ax2.set_yticklabels([10, 50, 200], fontsize=9)

bar_width = 0.12

x_baseline = x_positions - 1.5 * bar_width
x_baselineh = x_positions - 0.5 * bar_width
x_qiyana0 = x_positions + 0.5 * bar_width
x_qiyana1 = x_positions + 1.5 * bar_width

bars1 = ax2.bar(
    x_baseline, comm_baseline,
    width=bar_width,
    color=myred,
    edgecolor='black',
    linewidth=0.5,
    alpha=0.7,
    label='Comm-Baseline',
    zorder=1
)

bars2 = ax2.bar(
    x_baselineh, comm_baselineh,
    width=bar_width,
    color=mypurple,
    edgecolor='black',
    linewidth=0.5,
    alpha=0.7,
    label='Comm-BaselineH',
    zorder=1
)

bars3 = ax2.bar(
    x_qiyana0, comm_qiyana0,
    width=bar_width,
    color=myblue,
    edgecolor='black',
    linewidth=0.5,
    alpha=0.7,
    label='Comm-Qiyana0',
    zorder=1
)

bars4 = ax2.bar(
    x_qiyana1, comm_qiyana1,
    width=bar_width,
    color=myyellow,
    edgecolor='black',
    linewidth=0.5,
    alpha=0.7,
    label='Comm-Qiyana1',
    zorder=1
)

from matplotlib.lines import Line2D
legend_elements = [
    Line2D(
        [0], [0], color=c_baseline, lw=1,
        marker='o', markersize=2, label='Baseline'
    ),
    Line2D(
        [0], [0], color=c_baselineh, lw=1,
        marker='s', markersize=2, label='BaselineH'
    ),
    Line2D(
        [0], [0], color=c_qiyana0, lw=1,
        marker='x', markersize=2,
        label='Qiyana' + r'$_0$'
    ),
    Line2D(
        [0], [0], color=c_qiyana1, lw=1,
        marker='*', markersize=2,
        label='Qiyana' + r'$_1$'
    ),
]

ax1.legend(handles=legend_elements, ncol=2, loc='upper left', 
           fontsize=7.3, frameon=True, framealpha=0.95, columnspacing=0.1,
           handlelength=0.8, handletextpad=0.1, borderpad=0.1, labelspacing=0.1)  

legend_elements2 = [
    plt.Rectangle(
        (0, 0), 1, 1,
        facecolor=myred, edgecolor='black',
        linewidth=0.4, label='Baseline'
    ),
    plt.Rectangle(
        (0, 0), 1, 1,
        facecolor=mypurple, edgecolor='black',
        linewidth=0.4, label='BaselineH'
    ),
    plt.Rectangle(
        (0, 0), 1, 1,
        facecolor=myblue, edgecolor='black',
        linewidth=0.4,
        label='Qiyana' + r'$_0$'
    ),
    plt.Rectangle(
        (0, 0), 1, 1,
        facecolor=myyellow, edgecolor='black',
        linewidth=0.4,
        label='Qiyana' + r'$_1$'
    ),
]

ax2.legend(handles=legend_elements2, ncol=2, loc='upper right', 
           fontsize=7.3, frameon=True, framealpha=0.95, columnspacing=0.1,
           handlelength=0.8, handletextpad=0.1, borderpad=0.1, labelspacing=0.1)  

plt.tight_layout()
plt.savefig('lnlo.pdf', dpi=600, bbox_inches='tight')
plt.show()