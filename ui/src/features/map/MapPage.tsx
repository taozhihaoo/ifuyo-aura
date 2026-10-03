// Map 页面壳（§95–§97）：正式视觉语言的未来入口；不伪造 clusters/graph。
import { useTranslation } from '../../i18n/react';

export function MapPage() {
  const { t } = useTranslation();
  return (
    <section className="map-page" aria-label={t('map.title')}>
      <h1>{t('map.title')}</h1>
      <p className="map-intro">{t('map.intro')}</p>
      <div className="map-preview" aria-hidden>
        <div className="map-blob b1" />
        <div className="map-blob b2" />
        <div className="map-blob b3" />
      </div>
      <p className="map-status">{t('map.notEnabled')}</p>
    </section>
  );
}
