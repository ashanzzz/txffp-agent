import React, { useState } from 'react';
import { api, InvoicePreview } from '../api/client';
import { Calculator, FileText, CheckCircle2 } from 'lucide-react';

export const Invoice: React.FC = () => {
  const [startDate, setStartDate] = useState('2026-09-01');
  const [endDate, setEndDate] = useState('2026-09-30');
  const [preview, setPreview] = useState<InvoicePreview | null>(null);
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
    const confirmed = window.confirm(
      `请确认开票申请：\n日期：${startDate} 至 ${endDate}\n笔数：${preview.invoiceable_records} 笔\n金额：¥${preview.total_amount}\n\n该操作将向票根网提交正式开票，确认继续？`
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
        alert(res.error?.message || '开票失败');
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
        <p className="text-slate-500 text-sm mt-1">按日期筛选通行记录并进行开票预览（预览操作绝不产生发票）</p>
      </div>

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
              className="w-full bg-blue-600 hover:bg-blue-700 text-white font-medium py-2 px-4 rounded text-sm transition flex items-center justify-center gap-2"
            >
              <Calculator className="w-4 h-4" />
              {loading ? '正在查询...' : '查询并生成预览'}
            </button>
          </div>
        </div>
      </div>

      {preview && (
        <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-sm space-y-4">
          <div className="flex justify-between items-center border-b border-slate-100 pb-4">
            <div>
              <h2 className="text-lg font-bold text-slate-800">开票记录预览</h2>
              <div className="text-xs text-slate-500 mt-0.5">
                抬头: <span className="font-semibold text-slate-700">{preview.title_name || '个人'}</span>
              </div>
            </div>
            <div className="flex items-center gap-6">
              <div className="text-right">
                <div className="text-xs text-slate-400">待开笔数</div>
                <div className="text-lg font-bold text-slate-800">{preview.invoiceable_records} 笔</div>
              </div>
              <div className="text-right">
                <div className="text-xs text-slate-400">总金额</div>
                <div className="text-xl font-bold text-blue-600">¥{preview.total_amount}</div>
              </div>
              <button
                onClick={handleSubmit}
                disabled={submitting || preview.invoiceable_records === 0}
                className="bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 text-white font-medium py-2 px-5 rounded text-sm transition flex items-center gap-2 shadow-sm"
              >
                <FileText className="w-4 h-4" />
                {submitting ? '正在提交...' : '确认开票'}
              </button>
            </div>
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
                {preview.records.map((r) => (
                  <tr key={r.record_id} className="hover:bg-slate-50/50">
                    <td className="py-2.5 px-3 font-mono font-medium">{r.plate_number}</td>
                    <td className="py-2.5 px-3 text-xs">
                      <div>{r.en_time}</div>
                      <div className="text-slate-400">{r.en_station}</div>
                    </td>
                    <td className="py-2.5 px-3 text-xs">
                      <div>{r.ex_time}</div>
                      <div className="text-slate-400">{r.ex_station}</div>
                    </td>
                    <td className="py-2.5 px-3 text-right font-mono font-semibold text-slate-900">
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
      )}

      {submitResult && (
        <div className="bg-emerald-50 border border-emerald-200 rounded-lg p-5 flex items-start gap-4">
          <CheckCircle2 className="w-6 h-6 text-emerald-600 mt-0.5 flex-shrink-0" />
          <div>
            <h3 className="font-bold text-emerald-950">开票申请提交成功</h3>
            <p className="text-sm text-emerald-800 mt-1">
              申请单号：<span className="font-mono font-semibold">{submitResult.invoice_id}</span>，共包含{' '}
              {submitResult.record_count} 笔通行记录，总额 ¥{submitResult.total_amount}。
            </p>
          </div>
        </div>
      )}
    </div>
  );
};
