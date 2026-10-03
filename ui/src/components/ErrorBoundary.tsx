// §227–§229：feature 隔离错误边界。单页挂掉不白屏，其余页面与
// 控制条仍可用；重试仅重挂载本 feature，不影响全局 shell。
import { Component, type ReactNode } from 'react';

interface FeatureBoundaryProps {
  /** 诊断用途的 feature 名（不渲染给普通用户）。 */
  name: string;
  message: string;
  retryLabel: string;
  children: ReactNode;
}

interface FeatureBoundaryState {
  error: Error | null;
}

export class FeatureBoundary extends Component<FeatureBoundaryProps, FeatureBoundaryState> {
  override state: FeatureBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): FeatureBoundaryState {
    return { error };
  }

  override render() {
    if (this.state.error !== null) {
      return (
        <div className="feature-error" role="alert" data-feature={this.props.name}>
          <p>{this.props.message}</p>
          <button
            className="ctl-btn"
            onClick={() => this.setState({ error: null })}
          >
            {this.props.retryLabel}
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
