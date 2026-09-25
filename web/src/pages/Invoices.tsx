import React, { useEffect, useState } from 'react';
import { api, InvoiceItem } from '../api/client';
import { Download, FileCheck, RefreshCw, Car } from 'lucide-react';

export const Invoices: React.FC = () => {
  const [invoices, setInvoices] = useState<InvoiceItem[]>([]);
  const [loading, setLoading] = useState(false);

  const loadInvoices = async () => {
    setLoading(true);
    try {
      const res = await api.getInvoices();
      if (res.ok && res.data) {
        setInvoices(res.data);
      }
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadInvoices();
  }, []);

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">已开具发票历史</h1>
          <p className="text-slate-500 text-sm mt-1">查看历史已成功开具的发票批次，并下载 PDF 及 Excel 对账汇总单</p>
        </div>
        <button
          onClick={loadInvoices}
          disabled={loading}
          className="flex items-center gap-2 bg-white border border-slate-200 px-3 py-1.5 rounded-md text-sm font-medium hover:bg-slate-50 text-slate-700 shadow-sm"
        >
          <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
          刷新列表
        </button>
      </div>

      <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-sm">
        <div className="overflow-x-auto">
          <table className="w-full text-left text-sm">
            <thead className="bg-slate-50 text-slate-500 text-xs uppercase font-medium">
              <tr>
                <th className="py-2.5 px-3">开票批次 / 发票代码与号码</th>
                <th className="py-2.5 px-3">对应车辆</th>
                <th className="py-2.5 px-3">开票时间</th>
                <th className="py-2.5 px-3 text-right">金额 (元)</th>
                <th className="py-2.5 px-3 text-center">状态</th>
                <th className="py-2.5 px-3 text-right">下载操作</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100">
              {invoices.map((inv, idx) => {
                const plate = idx % 3 === 0 ? '津AF31581' : idx % 3 === 1 ? '津AAP3278' : '津C358P7';
                return (
                  <tr key={inv.invoice_id} className="hover:bg-slate-50/50">
                    <td className="py-3 px-3">
                      <div className="font-mono font-medium text-slate-900">{inv.invoice_id}</div>
                      <div className="text-xs text-slate-400 mt-0.5">
                        代码: <span className="font-mono text-slate-600">{inv.invoice_code || '-'}</span> | 号码:{' '}
                        <span className="font-mono text-slate-600">{inv.invoice_number || '-'}</span>
                      </div>
                    </td>
                    <td className="py-3 px-3">
                      <span className="inline-flex items-center gap-1 bg-slate-100 text-slate-800 text-xs px-2 py-0.5 rounded font-mono font-medium">
                        <Car className="w-3 h-3 text-slate-500" />
                        {plate}
                      </span>
                    </td>
                    <td className="py-3 px-3 text-slate-600 font-mono text-xs">{inv.issue_date}</td>
                    <td className="py-3 px-3 text-right font-mono font-bold text-slate-900 text-base">
                      ¥{inv.amount}
                    </td>
                    <td className="py-3 px-3 text-center">
                      <span className="bg-emerald-50 text-emerald-700 text-xs px-2 py-0.5 rounded font-medium flex items-center justify-center gap-1 w-max mx-auto">
                        <FileCheck className="w-3.5 h-3.5" />
                        {inv.status}
                      </span>
                    </td>
                    <td className="py-3 px-3 text-right">
                      <div className="flex items-center justify-end gap-2">
                        {inv.pdf_download_url && (
                          <a
                            href={inv.pdf_download_url}
                            target="_blank"
                            rel="noreferrer"
                            className="bg-blue-50 text-blue-700 hover:bg-blue-100 text-xs font-medium px-2.5 py-1 rounded inline-flex items-center gap-1"
                          >
                            <Download className="w-3 h-3" />
                            发票 PDF
                          </a>
                        )}
                        {inv.summary_download_url && (
                          <a
                            href={inv.summary_download_url}
                            target="_blank"
                            rel="noreferrer"
                            className="bg-slate-100 text-slate-700 hover:bg-slate-200 text-xs font-medium px-2.5 py-1 rounded inline-flex items-center gap-1"
                          >
                            <Download className="w-3 h-3" />
                            汇总单
                          </a>
                        )}
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};
