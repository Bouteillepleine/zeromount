import { t } from '../../lib/i18n';
import "./Header.css";

export function Header() {
  return (
    <header class="header">
      <h1 class="header__title">{t('header.title')}</h1>
      <span class="header__subtitle">{t('header.subtitle')}</span>
    </header>
  );
}
