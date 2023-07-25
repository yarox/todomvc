function selectTab(tabId) {
  document.getElementById(tabId).classList.add('is-active');
  document.querySelectorAll('.panel-tabs > a').forEach(e => {
    if (e.id !== tabId && e.classList.contains('is-active')) {
      e.classList.remove('is-active');
    }
  });
}
