import React, { useState } from 'react';
import { api, InvoicePreview, TransactionRecord } from '../api/client';
import { Calculator, FileText, CheckCircle2, Car, Layers } from 'lucide-react';

interface PlateSummary {
  plate: string;
  count: number;
  total: number;
  records: TransactionRecord[];
}

export const Invoice: React.FC = () => {
  const [startDate, setStartDate] = useState('2026-09-01');
  const [endDate, setEndDate] = useState('2026-09-30');
  const [preview, setPreview] = useState<InvoicePreview | null>(null);
  const [selectedPlate, setSelectedPlate] = useState<string>('ALL');
  const [loading, setLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [submitResult, setSubmitResult] = useState<any>(null);

  const handlePreview = async () => {
    setLoading(true);
    setSubmitResult(null);
    try {
      const res = await api.previewInvoice({ start_date: startDate, end_date: endDate });
      if (res.ok && res.data) {
        setPreview(res.data);
      } else {
        alert(res.error?.message || '查询失败');
      }
    } catch (e: any) {
      alert('请求失败: ' + e.message);
    } finally {
      setLoading(false);
    }
  };

  const handleSubmit = async () => {
    if (!preview || preview.invoiceable_records === 0) return;

    const plateText = Object.entries(plateSummaries)
      .map(([plate, s]) => `• 车牌 ${plate}: ${s.count} 笔, 合计 ¥${(s.total / 100).toFixed(2)}`)
      .join('\n');

    const confirmed = window.confirm(
      `【开票申请核对】\n开票区间：${startDate} 至 ${endDate}\n总笔数：${preview.invoiceable_records} 笔\n总金额：¥${preview.total_amount}\n\n车牌分类汇总：\n${plateText}\n\n严格安全模式提醒：确认后仅在系统内部生成核验记录，绝不向税局接口真实提交出票。确认继续？`
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
        setSubmitResult(res.data);
      } else {
        alert(res.error?.message || '核验提交失败');
      }
    } catch (e: any) {
      alert('请求失败: ' + e.message);
    } finally {
      setSubmitting(false);
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

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">开票与通行记录</h1>
        <p className="text-slate-500 text-sm mt-1">
          支持多车牌自动归类汇总、合并计算与开票核对（严格遵守安全规则：不向税局真实出票）
        </p>
      </div>

      {/* Query Control Bar */}
      <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-sm space-y-4">
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
              {loading ? '正在查询全卡记录...' : '查询并生成预览'}
            </button>
          </div>
        </div>
      </div>

      {preview && (
        <div className="space-y-6">
          {/* Per-Plate Summary Cards */}
          <div>
            <div className="flex items-center gap-2 mb-3">
              <Car className="w-5 h-5 text-blue-600" />
              <h2 className="text-base font-bold text-slate-800">各车牌分类汇总</h2>
              <span className="text-xs text-slate-400">点击卡片可快速筛选车辆</span>
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
              {/* Grand Total Card */}
              <div
                onClick={() => setSelectedPlate('ALL')}
                className={`cursor-pointer rounded-lg p-4 border transition ${
                  selectedPlate === 'ALL'
                    ? 'bg-blue-50/70 border-blue-500 shadow-sm'
                    : 'bg-white border-slate-200 hover:border-slate-300'
                }`}
              >
                <div className="flex items-center justify-between text-xs text-slate-500">
                  <span className="font-semibold uppercase tracking-wider flex items-center gap-1">
                    <Layers className="w-3.5 h-3.5 text-blue-600" />
                    全账户合并
                  </span>
                  <span className="bg-blue-100 text-blue-800 text-[11px] px-1.5 py-0.5 rounded font-medium">全部车辆</span>
                </div>
                <div className="mt-2 text-2xl font-bold text-blue-600">¥{preview.total_amount}</div>
                <div className="text-xs text-slate-500 mt-1">共 {preview.invoiceable_records} 笔行程待开票</div>
              </div>

              {/* Individual Plate Cards */}
              {plates.map((plate) => {
                const s = plateSummaries[plate];
                const isSelected = selectedPlate === plate;
                return (
                  <div
                    key={plate}
                    onClick={() => setSelectedPlate(plate)}
                    className={`cursor-pointer rounded-lg p-4 border transition ${
                      isSelected
                        ? 'bg-blue-50/70 border-blue-500 shadow-sm'
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
                      <span className="text-blue-600 font-medium">{isSelected ? '✓ 已选中' : '点击筛选'}</span>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Table Container */}
          <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-sm space-y-4">
            <div className="flex flex-col sm:flex-row justify-between sm:items-center gap-3 border-b border-slate-100 pb-4">
              <div>
                <div className="flex items-center gap-2">
                  <h3 className="text-lg font-bold text-slate-800">
                    {selectedPlate === 'ALL' ? '全部行程明细' : `车牌 ${selectedPlate} 行程明细`}
                  </h3>
                  <span className="text-xs bg-slate-100 text-slate-600 px-2 py-0.5 rounded font-medium">
                    当前展示 {displayedRecords.length} 笔
                  </span>
                </div>
                <div className="text-xs text-slate-500 mt-0.5">
                  企业抬头: <span className="font-semibold text-slate-700">{preview.title_name || '天津祺富机械加工有限公司'}</span>
                </div>
              </div>

              <div className="flex items-center gap-4">
                <div className="text-right">
                  <div className="text-xs text-slate-400">当前筛选金额</div>
                  <div className="text-xl font-bold text-blue-600">
                    ¥{selectedPlate === 'ALL'
                      ? preview.total_amount
                      : ((plateSummaries[selectedPlate]?.total || 0) / 100).toFixed(2)}
                  </div>
                </div>

                <button
                  onClick={handleSubmit}
                  disabled={submitting || displayedRecords.length === 0}
                  className="bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 text-white font-medium py-2 px-5 rounded text-sm transition flex items-center gap-2 shadow-sm"
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

            <div className="overflow-x-auto">
              <table className="w-full text-left text-sm">
                <thead className="bg-slate-50 text-slate-500 text-xs uppercase font-medium">
                  <tr>
                    <th className="py-2.5 px-3">车牌号</th>
                    <th className="py-2.5 px-3">入口时间 / 站点</th>
                    <th className="py-2.5 px-3">出口时间 / 站点</th>
                    <th className="py-2.5 px-3 text-right">金额 (元)</th>
                    <th className="py-2.5 px-3 text-center">状态</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100">
                  {displayedRecords.map((r) => (
                    <tr key={r.record_id} className="hover:bg-slate-50/50">
                      <td className="py-2.5 px-3 font-mono font-bold text-slate-900">
                        <span className="bg-slate-100 px-2 py-0.5 rounded">{r.plate_number}</span>
                      </td>
                      <td className="py-2.5 px-3 text-xs">
                        <div className="font-mono text-slate-700">{r.en_time}</div>
                        <div className="text-slate-400 mt-0.5">{r.en_station}</div>
                      </td>
                      <td className="py-2.5 px-3 text-xs">
                        <div className="font-mono text-slate-700">{r.ex_time}</div>
                        <div className="text-slate-400 mt-0.5">{r.ex_station}</div>
                      </td>
                      <td className="py-2.5 px-3 text-right font-mono font-bold text-slate-900">
                        ¥{r.amount}
                      </td>
                      <td className="py-2.5 px-3 text-center">
                        <span className="bg-amber-50 text-amber-700 text-xs px-2 py-0.5 rounded font-medium">
                          {r.invoice_status}
                        </span>
                      </td>
                    </tr>
                  ))}
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
            <h3 className="font-bold text-emerald-950">开票核验申请确认成功</h3>
            <p className="text-sm text-emerald-800 mt-1">
              申请批次：<span className="font-mono font-semibold">{submitResult.invoice_id}</span>，共包含{' '}
              {submitResult.record_count} 笔通行记录，总额 ¥{submitResult.total_amount}。
            </p>
            <p className="text-xs text-emerald-700 mt-2">
              安全提示：系统已完成开票金额与车牌参数的完整前置校验，未向票根税局接口做真实发票出票。
            </p>
          </div>
        </div>
      )}
    </div>
  );
};
