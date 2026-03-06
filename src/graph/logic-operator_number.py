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
qiyana_and = [925.02, 922.49, 926.78]
qiyanawosel_and = [134.97+3.78, 137.88+3.78, 136.55+3.78]

baseline_or = [10450.67+3.78, 15401.97+3.78, 20388.71+3.78]
qiyana_or = [1157.88, 1404.11, 1610.51]
qiyanawosel_or = [364.23+3.78, 591.23+3.78, 844.98+3.78]

comm_x = [1, 2, 3, 4, 5, 6]
comm_baseline = [262.77, 264.78, 266.79, 262.77, 264.78, 266.79]
comm_qiyana = [13.29, 13.29, 13.29, 21.58, 29.87, 38.16]
comm_qiyanawosel = [142.04, 142.04, 142.04, 150.33, 158.62, 166.91]

c_baseline = '#d62728'
c_qiyana = '#1f77b4'
c_qiyanawosel = '#ff7f0e'
myblue = '#B3CEFF'
myyellow = '#FFEBB7'
myred = '#E4A9A4'

fig, ax1 = plt.subplots(figsize=(5, 2))

ax1.set_facecolor('none') 
ax1.set_ylabel('Latency (s)', fontsize=10, family='Times New Roman', labelpad=0)
ax1.set_ylim(50, 500000)
ax1.set_xlim(0.5, 6.5)
ax1.set_yscale('log')
ax1.set_yticks([100, 1000, 10000, 100000])
ax1.set_yticklabels([100, 1000, 10000, 100000], fontsize=8)

ax1.set_xticks(x_positions)
ax1.set_xticklabels(x_labels, ha='center', fontsize=10, family='Times New Roman')

x_and = x_positions[:4]

ax1.plot(x_positions[:3], baseline_and, marker='o', markersize=4, 
         color=c_baseline, linewidth=1.5, label='Baseline (AND)', zorder=10)

ax1.plot(x_positions[:3], qiyana_and, marker='x', markersize=4, 
         color=c_qiyana, linewidth=1.5, label='Qiyana (AND)', zorder=10)

ax1.plot(x_positions[:3], qiyanawosel_and, marker='*', markersize=4, 
         color=c_qiyanawosel, linewidth=1.5, label='Qiyana-wosel (AND)', zorder=10)

ax1.plot(x_positions[3:], baseline_or, marker='o', markersize=4, 
         color=c_baseline, linewidth=1.5, linestyle='--', 
         label='Baseline (OR)', zorder=10)

ax1.plot(x_positions[3:], qiyana_or, marker='x', markersize=4, 
         color=c_qiyana, linewidth=1.5, linestyle='--', 
         label='Qiyana (OR)', zorder=10)

ax1.plot(x_positions[3:], qiyanawosel_or, marker='*', markersize=4, 
         color=c_qiyanawosel, linewidth=1.5, linestyle='--', 
         label='Qiyana-wosel (OR)', zorder=10)

def add_ratio_arrow(ax, x_pos, y_low, y_high, offset_x=0):
    ratio = y_high / y_low
    
    x_arrow = x_pos + offset_x
    
    ax.annotate('', xy=(x_arrow, y_high), xytext=(x_arrow, y_low),
                arrowprops=dict(arrowstyle='<->', color='black', lw=0.8), zorder=15)
    
    mid_y = np.sqrt(y_low * y_high)
    if offset_x < 0:
        ax.annotate(f'{ratio:.1f}x', xy=(x_arrow + offset_x, mid_y), 
                    ha='right', va='center', fontsize=6,
                    fontweight='bold', family='Times New Roman',
                    bbox=dict(boxstyle='round,pad=0.1', facecolor='white', alpha=0.7, edgecolor='none'),
                    zorder=20)
    else: 
        ax.annotate(f'{ratio:.1f}x', xy=(x_arrow + offset_x, mid_y), 
                    ha='left', va='center', fontsize=6,
                    fontweight='bold', family='Times New Roman',
                    bbox=dict(boxstyle='round,pad=0.1', facecolor='white', alpha=0.7, edgecolor='none'),
                    zorder=20)

add_ratio_arrow(ax1, 1, qiyana_and[0], baseline_and[0], offset_x=-0.06)
add_ratio_arrow(ax1, 1, qiyanawosel_and[0], baseline_and[0], offset_x=0.06)

add_ratio_arrow(ax1, 3, qiyana_and[2], baseline_and[2], offset_x=-0.06)
add_ratio_arrow(ax1, 3, qiyanawosel_and[2], baseline_and[2], offset_x=0.06)

add_ratio_arrow(ax1, 4, qiyana_or[0], baseline_or[0], offset_x=-0.06)
add_ratio_arrow(ax1, 4, qiyanawosel_or[0], baseline_or[0], offset_x=0.06)

add_ratio_arrow(ax1, 6, qiyana_or[2], baseline_or[2], offset_x=-0.06)
add_ratio_arrow(ax1, 6, qiyanawosel_or[2], baseline_or[2], offset_x=0.06)

ax1.axvline(x=3.5, color='red', linestyle='-.', linewidth=1.5)

ax1.text(3.25, 150000, 'AND Queries', color='red', fontsize=10, ha='right', 
         family='Times New Roman', fontweight='bold', 
         bbox=dict(boxstyle='round,pad=0.2', facecolor='white', alpha=0.8))
ax1.text(3.75, 150000, 'OR Queries', color='blue', fontsize=10, ha='left', 
         family='Times New Roman', fontweight='bold', 
         bbox=dict(boxstyle='round,pad=0.2', facecolor='white', alpha=0.8))

ax2 = ax1.twinx()
ax2.set_zorder(ax1.get_zorder() - 1)
ax2.set_ylabel('Communication (MB)', fontsize=10, family='Times New Roman', labelpad=0)
ax2.set_ylim(5, 1200)
ax2.set_yscale('log')
ax2.set_yticks([10, 50, 200])
ax2.set_yticklabels([10, 50, 200], fontsize=8)

bar_width = 0.1
x_baseline = x_positions - bar_width * 1.5
x_qiyana = x_positions
x_qiyanawosel = x_positions + bar_width * 1.5

bars1 = ax2.bar(x_baseline, comm_baseline, width=bar_width, color=myred, 
                edgecolor='black', linewidth=0.5, alpha=0.7, label='Comm-Baseline', zorder=1)

bars2 = ax2.bar(x_qiyana, comm_qiyana, width=bar_width, color=myblue, 
                edgecolor='black', linewidth=0.5, alpha=0.7, label='Comm-Qiyana', zorder=1)

bars3 = ax2.bar(x_qiyanawosel, comm_qiyanawosel, width=bar_width, color=myyellow, 
                edgecolor='black', linewidth=0.5, alpha=0.7, label='Comm-Qiyana-wosel', zorder=1)

from matplotlib.lines import Line2D
legend_elements = [
    Line2D([0], [0], color=c_baseline, lw=1, marker='o', markersize=2, label='Baseline'),
    Line2D([0], [0], color=c_qiyana, lw=1, marker='x', markersize=2, label='Qiyana'),
    Line2D([0], [0], color=c_qiyanawosel, lw=1, marker='*', markersize=2, label='Qiyana-wosel'),
]

ax1.legend(handles=legend_elements, ncol=1, loc='upper left', 
           fontsize=7, frameon=True, framealpha=0.95, 
           handlelength=0.8, handletextpad=0.2, borderpad=0.2, labelspacing=0.2)  

legend_elements2 = [
    plt.Rectangle((0,0),1,1, facecolor=myred, edgecolor='black', linewidth=0.4, label='Baseline'),
    plt.Rectangle((0,0),1,1, facecolor=myblue, edgecolor='black', linewidth=0.4, label='Qiyana'),
    plt.Rectangle((0,0),1,1, facecolor=myyellow, edgecolor='black', linewidth=0.4, label='Qiyana-wosel'),
]

ax2.legend(handles=legend_elements2, ncol=1, loc='upper right', 
           fontsize=7, frameon=True, framealpha=0.95, 
           handlelength=0.8, handletextpad=0.2, borderpad=0.2, labelspacing=0.2)  

ax1.tick_params(axis="both", which="major", direction="in", width=0.8, length=4, labelsize=9)
ax2.tick_params(axis="y", which="major", direction="in", width=0.8, length=4, labelsize=9)

plt.tight_layout()
plt.savefig('lnlo.pdf', dpi=600, bbox_inches='tight')
plt.show()
