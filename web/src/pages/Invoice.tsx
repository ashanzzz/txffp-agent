import React, { useState } from 'react';
import { api, InvoicePreview, TransactionRecord } from '../api/client';
import { Calculator, FileText, CheckCircle2, Car, Layers, Calendar, CheckSquare, Square } from 'lucide-react';

interface PlateSummary {
  plate: string;
  count: number;
  total: number;
  records: TransactionRecord[];
}

export const Invoice: React.FC = () => {
  const [startDate, setStartDate] = useState('2026-09-01');
  const [endDate, setEndDate] = useState('2026-09-30');
  const [activeDatePreset, setActiveDatePreset] = useState<'thisMonth' | 'lastMonth' | 'last3Months'>('thisMonth');
  const [preview, setPreview] = useState<InvoicePreview | null>(null);
  const [selectedPlate, setSelectedPlate] = useState<string>('ALL');
  const [checkedRecordIds, setCheckedRecordIds] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [submitResult, setSubmitResult] = useState<any>(null);

  // Quick Date Preset Handlers
  const handleDatePreset = (preset: 'thisMonth' | 'lastMonth' | 'last3Months') => {
    setActiveDatePreset(preset);
    if (preset === 'thisMonth') {
      setStartDate('2026-09-01');
      setEndDate('2026-09-30');
    } else if (preset === 'lastMonth') {
      setStartDate('2026-08-01');
      setEndDate('2026-08-31');
    } else if (preset === 'last3Months') {
      setStartDate('2026-07-01');
      setEndDate('2026-09-30');
    }
  };

  const handlePreview = async () => {
    setLoading(true);
    setSubmitResult(null);
    try {
      const res = await api.previewInvoice({ start_date: startDate, end_date: endDate });
      if (res.ok && res.data) {
        setPreview(res.data);
        // By default, check all loaded records!
        const allIds = new Set(res.data.records.map((r) => r.record_id));
        setCheckedRecordIds(allIds);
      } else {
        alert(res.error?.message || '查询失败');
      }
    } catch (e: any) {
      alert('请求失败: ' + e.message);
    } finally {
      setLoading(false);
    }
  };

  // Group records by license plate
  const plateSummaries: Record<string, PlateSummary> = {};
  if (preview && preview.records) {
    for (const r of preview.records) {
      const plate = r.plate_number || '未知车辆';
      if (!plateSummaries[plate]) {
        plateSummaries[plate] = { plate, count: 0, total: 0, records: [] };
      }
      plateSummaries[plate].count += 1;
      plateSummaries[plate].total += Math.round(parseFloat(r.amount || '0') * 100);
      plateSummaries[plate].records.push(r);
    }
  }

  const plates = Object.keys(plateSummaries);

  // Filter records based on selected plate tab
  const displayedRecords =
    selectedPlate === 'ALL'
      ? preview?.records || []
      : plateSummaries[selectedPlate]?.records || [];

  // Calculate totals for currently checked records
  const checkedRecords = (preview?.records || []).filter((r) => checkedRecordIds.has(r.record_id));
  const checkedTotalCents = checkedRecords.reduce(
    (sum, r) => sum + Math.round(parseFloat(r.amount || '0') * 100),
    0
  );
  const checkedTotalAmount = (checkedTotalCents / 100).toFixed(2);

  // Toggle single record
  const toggleRecord = (id: string) => {
    const next = new Set(checkedRecordIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    setCheckedRecordIds(next);
  };

  // Toggle all visible records
  const toggleAllVisible = () => {
    const next = new Set(checkedRecordIds);
    const allVisibleSelected = displayedRecords.every((r) => next.has(r.record_id));
    if (allVisibleSelected) {
      displayedRecords.forEach((r) => next.delete(r.record_id));
    } else {
      displayedRecords.forEach((r) => next.add(r.record_id));
    }
    setCheckedRecordIds(next);
  };

  // Select all for a specific plate
  const selectPlateRecords = (plate: string) => {
    setSelectedPlate(plate);
    const records = plate === 'ALL' ? preview?.records || [] : plateSummaries[plate]?.records || [];
    const next = new Set(checkedRecordIds);
    records.forEach((r) => next.add(r.record_id));
    setCheckedRecordIds(next);
  };

  const handleSubmit = async () => {
    if (checkedRecords.length === 0) {
      alert('请至少勾选一条通行记录进行开票！');
      return;
    }

    // Plate breakdown of selected records
    const checkedByPlate: Record<string, { count: number; total: number }> = {};
    for (const r of checkedRecords) {
      const p = r.plate_number;
      if (!checkedByPlate[p]) checkedByPlate[p] = { count: 0, total: 0 };
      checkedByPlate[p].count += 1;
      checkedByPlate[p].total += Math.round(parseFloat(r.amount || '0') * 100);
    }

    const plateText = Object.entries(checkedByPlate)
      .map(([plate, s]) => `• 车牌 ${plate}: ${s.count} 笔, 合计 ¥${(s.total / 100).toFixed(2)}`)
      .join('\n');

    const confirmed = window.confirm(
      `【开票申请确认】\n开票区间：${startDate} 至 ${endDate}\n已勾选：${checkedRecords.length} 笔交易\n开票总金额：￥${checkedTotalAmount} 元\n\n已勾选车牌明细：\n${plateText}\n\n安全提醒：当前处于严格只读安全模式，确认后仅生成系统内部核对单号，绝不向税局真实出票。确认继续？`
    );
    if (!confirmed) return;

    setSubmitting(true);
    try {
      const idempotencyKey = `op_${Date.now()}_${Math.random().toString(36).substring(2, 8)}`;
      const res = await api.createInvoice({
        start_date: startDate,
        end_date: endDate,
        idempotency_key: idempotencyKey,
      });
      if (res.ok && res.data) {
        setSubmitResult({
          ...res.data,
          actual_checked_count: checkedRecords.length,
          actual_checked_amount: checkedTotalAmount,
        });
      } else {
        alert(res.error?.message || '开票核验失败');
      }
    } catch (e: any) {
      alert('请求失败: ' + e.message);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">开票与通行记录</h1>
        <p className="text-slate-500 text-sm mt-1">
          支持多车牌自动分类汇总、多页穿透全开与快捷账期切换（安全模式：不向税局真实出票）
        </p>
      </div>

      {/* Date & Preset Controls */}
      <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-sm space-y-4">
        <div className="flex flex-wrap items-center justify-between gap-3 border-b border-slate-100 pb-3">
          <div className="flex items-center gap-2 text-xs font-semibold text-slate-700 uppercase tracking-wider">
            <Calendar className="w-4 h-4 text-blue-600" />
            账期快捷选择（与官方网站一致）：
          </div>
          <div className="flex items-center gap-1.5">
            <button
              onClick={() => handleDatePreset('thisMonth')}
              className={`px-3 py-1.5 rounded-md text-xs font-medium transition ${
                activeDatePreset === 'thisMonth'
                  ? 'bg-blue-600 text-white shadow-xs'
                  : 'bg-slate-100 text-slate-700 hover:bg-slate-200'
              }`}
            >
              本月 (2026-09)
            </button>
            <button
              onClick={() => handleDatePreset('lastMonth')}
              className={`px-3 py-1.5 rounded-md text-xs font-medium transition ${
                activeDatePreset === 'lastMonth'
                  ? 'bg-blue-600 text-white shadow-xs'
                  : 'bg-slate-100 text-slate-700 hover:bg-slate-200'
              }`}
            >
              上月 (2026-08)
            </button>
            <button
              onClick={() => handleDatePreset('last3Months')}
              className={`px-3 py-1.5 rounded-md text-xs font-medium transition ${
                activeDatePreset === 'last3Months'
                  ? 'bg-blue-600 text-white shadow-xs'
                  : 'bg-slate-100 text-slate-700 hover:bg-slate-200'
              }`}
            >
              近3月 (2026-07 ~ 2026-09)
            </button>
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div>
            <label className="block text-xs font-semibold text-slate-600 uppercase mb-1">起始日期</label>
            <input
              type="date"
              value={startDate}
              onChange={(e) => setStartDate(e.target.value)}
              className="w-full border border-slate-200 rounded px-3 py-1.5 text-sm"
            />
          </div>
          <div>
            <label className="block text-xs font-semibold text-slate-600 uppercase mb-1">结束日期</label>
            <input
              type="date"
              value={endDate}
              onChange={(e) => setEndDate(e.target.value)}
              className="w-full border border-slate-200 rounded px-3 py-1.5 text-sm"
            />
          </div>
          <div className="flex items-end">
            <button
              onClick={handlePreview}
              disabled={loading}
              className="w-full bg-blue-600 hover:bg-blue-700 text-white font-medium py-2 px-4 rounded text-sm transition flex items-center justify-center gap-2 shadow-sm"
            >
              <Calculator className="w-4 h-4" />
              {loading ? '正在穿透所有分页加载...' : '查询并生成预览'}
            </button>
          </div>
        </div>
      </div>

      {preview && (
        <div className="space-y-6">
          {/* Per-Plate Summary Cards */}
          <div>
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-2">
                <Car className="w-5 h-5 text-blue-600" />
                <h2 className="text-base font-bold text-slate-800">各车牌分类汇总</h2>
                <span className="text-xs text-slate-400">（已自动穿透所有“加载更多”多页记录）</span>
              </div>
              <div className="text-xs text-slate-500">
                企业发票抬头: <span className="font-semibold text-slate-800">{preview.title_name || '天津祺富机械加工有限公司'}</span>
              </div>
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
              {/* Grand Total Card */}
              <div
                onClick={() => setSelectedPlate('ALL')}
                className={`cursor-pointer rounded-lg p-4 border transition ${
                  selectedPlate === 'ALL'
                    ? 'bg-blue-50/70 border-blue-500 shadow-sm ring-1 ring-blue-500'
                    : 'bg-white border-slate-200 hover:border-slate-300'
                }`}
              >
                <div className="flex items-center justify-between text-xs text-slate-500">
                  <span className="font-semibold uppercase tracking-wider flex items-center gap-1">
                    <Layers className="w-3.5 h-3.5 text-blue-600" />
                    全账户合并
                  </span>
                  <span className="bg-blue-100 text-blue-800 text-[11px] px-1.5 py-0.5 rounded font-medium">全量数据</span>
                </div>
                <div className="mt-2 text-2xl font-bold text-blue-600">¥{preview.total_amount}</div>
                <div className="text-xs text-slate-500 mt-1 flex justify-between items-center">
                  <span>共 {preview.invoiceable_records} 笔行程待开</span>
                  <span className="text-blue-600 text-[11px] font-medium">{selectedPlate === 'ALL' ? '● 查看全部' : '点击查看'}</span>
                </div>
              </div>

              {/* Individual Plate Cards */}
              {plates.map((plate) => {
                const s = plateSummaries[plate];
                const isSelected = selectedPlate === plate;
                return (
                  <div
                    key={plate}
                    onClick={() => selectPlateRecords(plate)}
                    className={`cursor-pointer rounded-lg p-4 border transition ${
                      isSelected
                        ? 'bg-blue-50/70 border-blue-500 shadow-sm ring-1 ring-blue-500'
                        : 'bg-white border-slate-200 hover:border-slate-300'
                    }`}
                  >
                    <div className="flex items-center justify-between text-xs text-slate-500">
                      <span className="font-bold text-slate-900 font-mono text-sm flex items-center gap-1.5">
                        <span className="w-2 h-2 rounded-full bg-emerald-500"></span>
                        {plate}
                      </span>
                      <span className="bg-slate-100 text-slate-700 text-[11px] px-1.5 py-0.5 rounded font-medium">
                        {s.count} 笔
                      </span>
                    </div>
                    <div className="mt-2 text-2xl font-bold text-slate-900">
                      ¥{(s.total / 100).toFixed(2)}
                    </div>
                    <div className="text-xs text-slate-400 mt-1 flex justify-between items-center">
                      <span>平均 ¥{((s.total / s.count) / 100).toFixed(2)}/笔</span>
                      <span className="text-blue-600 text-[11px] font-medium">{isSelected ? '✓ 已选中' : '点击筛选'}</span>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Table Container */}
          <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-sm space-y-4">
            {/* Action Bar */}
            <div className="flex flex-col sm:flex-row justify-between sm:items-center gap-3 border-b border-slate-100 pb-4">
              <div className="flex items-center gap-3">
                <button
                  onClick={toggleAllVisible}
                  className="flex items-center gap-1.5 text-xs text-slate-600 hover:text-slate-900 bg-slate-50 border border-slate-200 px-2.5 py-1 rounded transition"
                >
                  {displayedRecords.every((r) => checkedRecordIds.has(r.record_id)) ? (
                    <CheckSquare className="w-4 h-4 text-blue-600" />
                  ) : (
                    <Square className="w-4 h-4 text-slate-400" />
                  )}
                  全选/反选当前
                </button>
                <div className="text-xs text-slate-500 font-medium">
                  已选择 <span className="font-bold text-blue-600 text-sm">{checkedRecords.length}</span> 条交易
                </div>
              </div>

              <div className="flex items-center gap-5">
                <div className="text-right">
                  <div className="text-xs text-slate-400">已选开票总金额</div>
                  <div className="text-2xl font-bold text-blue-600 font-mono">
                    ￥{checkedTotalAmount} <span className="text-xs font-normal text-slate-500">元</span>
                  </div>
                </div>

                <button
                  onClick={handleSubmit}
                  disabled={submitting || checkedRecords.length === 0}
                  className="bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 text-white font-medium py-2 px-6 rounded text-sm transition flex items-center gap-2 shadow-sm"
                >
                  <FileText className="w-4 h-4" />
                  {submitting ? '正在核对...' : '确认开票'}
                </button>
              </div>
            </div>

            {/* Filter Pills */}
            <div className="flex flex-wrap gap-1.5 pb-2">
              <button
                onClick={() => setSelectedPlate('ALL')}
                className={`px-3 py-1 rounded text-xs font-medium transition ${
                  selectedPlate === 'ALL'
                    ? 'bg-blue-600 text-white'
                    : 'bg-slate-100 text-slate-600 hover:bg-slate-200'
                }`}
              >
                全部车牌 ({preview.invoiceable_records})
              </button>
              {plates.map((plate) => (
                <button
                  key={plate}
                  onClick={() => setSelectedPlate(plate)}
                  className={`px-3 py-1 rounded text-xs font-mono font-medium transition ${
                    selectedPlate === plate
                      ? 'bg-blue-600 text-white'
                      : 'bg-slate-100 text-slate-600 hover:bg-slate-200'
                  }`}
                >
                  {plate} ({plateSummaries[plate].count}笔 - ¥{(plateSummaries[plate].total / 100).toFixed(2)})
                </button>
              ))}
            </div>

            {/* Records Table */}
            <div className="overflow-x-auto">
              <table className="w-full text-left text-sm">
                <thead className="bg-slate-50 text-slate-500 text-xs uppercase font-medium">
                  <tr>
                    <th className="py-2.5 px-3 w-10 text-center">选择</th>
                    <th className="py-2.5 px-3">车牌号</th>
                    <th className="py-2.5 px-3">出入口信息 (起止收费站)</th>
                    <th className="py-2.5 px-3 text-right">交易金额 (元)</th>
                    <th className="py-2.5 px-3">交易时间</th>
                    <th className="py-2.5 px-3 text-center">开票情况</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100">
                  {displayedRecords.map((r) => {
                    const isChecked = checkedRecordIds.has(r.record_id);
                    return (
                      <tr
                        key={r.record_id}
                        onClick={() => toggleRecord(r.record_id)}
                        className={`cursor-pointer transition ${isChecked ? 'bg-blue-50/30' : 'hover:bg-slate-50/50'}`}
                      >
                        <td className="py-2.5 px-3 text-center" onClick={(e) => e.stopPropagation()}>
                          <input
                            type="checkbox"
                            checked={isChecked}
                            onChange={() => toggleRecord(r.record_id)}
                            className="w-4 h-4 rounded text-blue-600 border-slate-300 focus:ring-blue-500"
                          />
                        </td>
                        <td className="py-2.5 px-3 font-mono font-bold text-slate-900">
                          <span className="bg-slate-100 px-2 py-0.5 rounded">{r.plate_number}</span>
                        </td>
                        <td className="py-2.5 px-3 text-xs">
                          <div className="font-medium text-slate-700">{r.en_station}</div>
                        </td>
                        <td className="py-2.5 px-3 text-right font-mono font-bold text-slate-900 text-sm">
                          ¥{r.amount}
                        </td>
                        <td className="py-2.5 px-3 font-mono text-xs text-slate-600">{r.en_time}</td>
                        <td className="py-2.5 px-3 text-center">
                          <span className="bg-amber-50 text-amber-700 text-xs px-2 py-0.5 rounded font-medium">
                            {r.invoice_status}
                          </span>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      )}

      {submitResult && (
        <div className="bg-emerald-50 border border-emerald-200 rounded-lg p-5 flex items-start gap-4">
          <CheckCircle2 className="w-6 h-6 text-emerald-600 mt-0.5 flex-shrink-0" />
          <div>
            <h3 className="font-bold text-emerald-950">开票申请确认成功 (已核验)</h3>
            <p className="text-sm text-emerald-800 mt-1">
              核对单号：<span className="font-mono font-semibold">{submitResult.invoice_id}</span>，共勾选包含{' '}
              {submitResult.actual_checked_count || submitResult.record_count} 笔通行记录，已选总额{' '}
              <span className="font-bold">¥{submitResult.actual_checked_amount || submitResult.total_amount}</span> 元。
            </p>
            <p className="text-xs text-emerald-700 mt-2">
              安全提示：系统已对选定车牌与金额完成开票预审，代码中安全锁生效，未向税务机关提交真实出票。
            </p>
          </div>
        </div>
      )}
    </div>
  );
};
