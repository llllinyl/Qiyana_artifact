import numpy as np
import matplotlib.pyplot as plt
from matplotlib import rcParams
from mpl_toolkits.axes_grid1.inset_locator import inset_axes
from matplotlib.patches import Patch, Rectangle, Circle, ConnectionPatch
from matplotlib.lines import Line2D
from matplotlib.offsetbox import OffsetImage, AnnotationBbox

# A AND B
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
qiyana01   = np.array([0, 0, 0, 0]) 
qiyana11   = np.array([2.04, 2.19, 3.04, 6.79]) 

coeus2 = np.array([0, 0, 0, 0]) 
baseline2  = np.array([10390.56, 43045.13, 172264.79, 675482.71]) 
qiyana02   = np.array([783.43, 3141.68, 13121.27, 50602.35]) 
qiyana12   = np.array([98.40, 393.31, 1615.89, 6288.92])

coeus3 = np.array([0.086, 0.110, 0.129, 0.214]) 
baseline3  = np.array([0.086, 0.110, 0.129, 0.214]) 
qiyana03   = np.array([0.086, 0.110, 0.129, 0.214]) 
qiyana13   = np.array([0.086, 0.110, 0.129, 0.214]) 

coeus4 = np.array([0.086, 0.144, 0.193, 0.408]) 
baseline4  = np.array([0.086, 0.144, 0.193, 0.408]) 
qiyana04   = np.array([0.086, 0.144, 0.193, 0.408]) 
qiyana14   = np.array([0.086, 0.144, 0.193, 0.408]) 

fig, (ax1, ax2) = plt.subplots(1, 2, sharey=True, figsize=(5, 3),
                               gridspec_kw={'width_ratios': [1, 35], 'wspace': 0.03})

c_qiyana1 = '#B3CEFF'
c_qiyana0 = '#FFEBB7'
c_baseline = '#E4A9A4'
c_coeus = '#B99EC6'

bar_width = 0.15

x_coeus = x_pos - 1.875*bar_width
x_baseline = x_pos - 0.625*bar_width
x_qiyana0 = x_pos + 0.625*bar_width
x_qiyana1 = x_pos + 1.875*bar_width


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

p9 = ax2.bar(x_qiyana0, qiyana01, bar_width,
            label='Qiyana0-CPU1', color=c_qiyana0, edgecolor='black', linewidth=0.5)
p10 = ax2.bar(x_qiyana0, qiyana02, bar_width, bottom=qiyana01,
            label='Qiyana0-CPU2', color=c_qiyana0, edgecolor='black', linewidth=0.5, hatch='x')
p11 = ax2.bar(x_qiyana0, qiyana03, bar_width, bottom=qiyana01 + qiyana02,
            label='Qiyana0-CPU3', color=c_qiyana0, edgecolor='black', linewidth=0.5, hatch='--')
p12 = ax2.bar(x_qiyana0, qiyana04, bar_width, bottom=qiyana01 + qiyana02 + qiyana03,
            label='Qiyana0-CPU4', color=c_qiyana0, edgecolor='black', linewidth=0.5, hatch='|||')

p13 = ax2.bar(x_qiyana1, qiyana11, bar_width,
            label='Qiyana1-CPU1', color=c_qiyana1, edgecolor='black', linewidth=0.5)
p14 = ax2.bar(x_qiyana1, qiyana12, bar_width, bottom=qiyana11,
            label='Qiyana1-CPU2', color=c_qiyana1, edgecolor='black', linewidth=0.5, hatch='x')
p15 = ax2.bar(x_qiyana1, qiyana13, bar_width, bottom=qiyana11 + qiyana12,
            label='Qiyana1-CPU3', color=c_qiyana1, edgecolor='black', linewidth=0.5, hatch='--')
p16 = ax2.bar(x_qiyana1, qiyana14, bar_width, bottom=qiyana11 + qiyana12 + qiyana13,
            label='Qiyana1-CPU4', color=c_qiyana1, edgecolor='black', linewidth=0.5, hatch='|||')

for i in range(len(x_pos)):
    values = [
        coeus1[i] + coeus2[i] + coeus3[i] + coeus4[i],
        baseline1[i] + baseline2[i] + baseline3[i] + baseline4[i],
        qiyana01[i] + qiyana02[i] + qiyana03[i] + qiyana04[i],
        qiyana11[i] + qiyana12[i] + qiyana13[i] + qiyana14[i]
    ]
    max_val = values[1]
    
    # total_coeus = coeus1[i] + coeus2[i]
    # ratio_coeus = int(round(max_val / total_coeus)) if total_coeus > 0 else 0
    # offset = 0.1 if i > 0 else 0.075
    # ax2.text(x_coeus[i] - offset, total_coeus * 1.1, f'{ratio_coeus}' + r'$\times$', 
    #          ha='center', va='bottom', fontsize=9, family='Times New Roman',fontweight='bold')
    
    ratio_qiyana0 = max_val / values[2]
    ax2.text(x_qiyana0[i] + 0.14, values[2] * 1.1, f'{ratio_qiyana0:.1f}' + r'$\times$', 
             ha='center', va='bottom', fontsize=12, family='Times New Roman',fontweight='bold')
    
    ratio_qiyana1 = max_val / values[3]
    ax2.text(x_qiyana1[i] + 0.16, values[3] * 1.1, f'{ratio_qiyana1:.1f}' + r'$\times$', 
             ha='center', va='bottom', fontsize=12, family='Times New Roman',fontweight='bold')


ax2.set_xlim(x_pos[0] - 3*bar_width, x_pos[-1] + 4.3*bar_width)
ax2.set_xticks(x_pos)
ax2.set_xticklabels(x_labels, fontsize=15)

legend_elements = [
    Patch(facecolor=c_coeus, edgecolor='black', linewidth=0.4, label='Coeus'),
    Patch(facecolor=c_baseline, edgecolor='black', linewidth=0.4, label='Baseline'),
    Patch(facecolor=c_qiyana0, edgecolor='black', linewidth=0.4, label='Qiyana'+ r'$_0$'),
    Patch(facecolor=c_qiyana1, edgecolor='black', linewidth=0.4, label='Qiyana'+ r'$_1$'),
]

ax2.legend(handles=legend_elements, ncol=2, loc='upper right',
    fontsize=11,
    frameon=True,
    framealpha=0.95,
    bbox_to_anchor=(0.99, 0.99),
           handlelength=0.8, handletextpad=0.2, borderpad=0.2, labelspacing=0.2)  

ax1.set_xlim(0, 1)
ax1.set_xticks([0])
ax1.set_xticklabels(['0'], fontsize=12)

ax1.set_ylim(1, 45000000)
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

axins = inset_axes(
    ax2,
    width="27.5%",
    height="27.5%",
    loc='upper left',
    bbox_to_anchor=(0.02, 0, 1, 1),
    bbox_transform=ax2.transAxes,
    borderpad=0.6
)

# 只画 Coeus 的 CPU3 和 CPU4（4个数据规模）
ins_w = 0.28
ins_x = np.arange(len(x_labels))

axins.bar(
    ins_x - ins_w/2, coeus3,
    width=ins_w,
    color=c_coeus,
    edgecolor='black',
    linewidth=0.5,
    hatch='---',
    label='MR'
)

axins.bar(
    ins_x + ins_w/2, coeus4,
    width=ins_w,
    color=c_coeus,
    edgecolor='black',
    linewidth=0.5,
    hatch='|||',
    label='DR'
)

axins.set_xticks(ins_x)
axins.set_xticklabels(x_labels, fontsize=8)
axins.tick_params(axis='y', labelsize=8)
# axins.set_ylabel('Latency (s)', fontsize=6)

# 因为数值很小，单独设置一个合适范围
ymax = max(np.max(coeus3), np.max(coeus4)) * 1.25
axins.set_ylim(0, ymax)

axins.legend(
    fontsize=8,
    loc='upper left',
    frameon=True,
    handlelength=1.0,
    borderpad=0.2
)

# =========================
# zoom indication: 框 + 两条线 + 放大镜
# =========================

# =========================
# zoom indication: 只框住 Coeus 的第一根柱子
# =========================

# Coeus 第一根柱子的位置和高度
i = 0
coeus_first_x = x_coeus[i]
coeus_first_h = coeus1[i] + coeus2[i] + coeus3[i] + coeus4[i]

# 1) 主图中的放大框：只框第一根 Coeus 柱子
zoom_x0 = coeus_first_x - bar_width * 0.6
zoom_w  = bar_width * 1.2
zoom_y0 = 1.05
zoom_y1 = coeus_first_h * 1.35

zoom_rect = Rectangle(
    (zoom_x0, zoom_y0),
    zoom_w,
    zoom_y1 - zoom_y0,
    fill=False,
    edgecolor='red',
    linewidth=0.8,
    linestyle='-',
    zorder=10
)
ax2.add_patch(zoom_rect)

# 2) 两条连接线：从这个框连到 inset
con1 = ConnectionPatch(
    xyA=(zoom_x0, zoom_y1),          # 主图框左上角
    coordsA=ax2.transData,
    xyB=(0, 0),                      # inset 左下角
    coordsB=axins.transAxes,
    axesA=ax2,
    axesB=axins,
    color='red',
    linewidth=0.8,
    clip_on=False,
    zorder=100
)

con2 = ConnectionPatch(
    xyA=(zoom_x0 + zoom_w, zoom_y1), # 主图框右上角
    coordsA=ax2.transData,
    xyB=(1, 0),                      # inset 右下角
    coordsB=axins.transAxes,
    axesA=ax2,
    axesB=axins,
    color='red',
    linewidth=0.8,
    clip_on=False,
    zorder=100
)

fig.add_artist(con1)
fig.add_artist(con2)

# 3) 放大镜图标，放在第一根 Coeus 柱子附近
mag_x, mag_y = 0.055, 0.12
mag_r = 0.022

img = plt.imread('放大镜.png')

# 创建图片对象，zoom 控制大小
imagebox = OffsetImage(img, zoom=0.08)

# 放到 ax2 中
ab = AnnotationBbox(
    imagebox,
    (0.02, 0.12),                 # 位置：ax2 的相对坐标
    xycoords=ax2.transAxes,
    frameon=False,
    zorder=20
)
ax2.add_artist(ab)

plt.savefig('a.pdf', dpi=600, bbox_inches='tight')
plt.show()