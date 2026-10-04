// Map 搜索 runner（§309/§345）：复用 M3 library_search（不在前端加载
// 10k 元数据自滤）；序列号守卫——旧 query 的迟到响应不得覆盖最新
// （§345/§485）。纯逻辑、可 node 环境测试。
export interface SearchRow {
  id: number;
}

export type SearchFn = (query: string, limit: number, offset: number) => Promise<SearchRow[]>;

export class LibrarySearchRunner {
  private seq = 0;
  private readonly search: SearchFn;
  private readonly onResult: (query: string, ids: number[]) => void;

  constructor(search: SearchFn, onResult: (query: string, ids: number[]) => void) {
    this.search = search;
    this.onResult = onResult;
  }

  run(query: string): void {
    const mine = ++this.seq;
    const q = query.trim();
    if (q === '') {
      this.onResult('', []);
      return;
    }
    void this.search(q, 50, 0).then(
      (rows) => {
        if (mine !== this.seq) return; // §345：stale response 丢弃
        this.onResult(q, rows.map((r) => r.id));
      },
      () => {
        if (mine === this.seq) this.onResult(q, []);
      },
    );
  }

  /** 最近的请求序号（测试/诊断）。 */
  get latest(): number {
    return this.seq;
  }
}
