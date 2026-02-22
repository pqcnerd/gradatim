const { default: pngToIco } = require('png-to-ico');
const fs = require('fs');
const path = require('path');

const src = path.join(__dirname, '..', 'build', 'icon.png');
const dest = path.join(__dirname, '..', 'build', 'icon.ico');

pngToIco(src)
  .then(buf => {
    fs.writeFileSync(dest, buf);
    console.log('Created build/icon.ico');
  })
  .catch(err => {
    console.error('Failed to create icon.ico:', err.message);
    process.exit(1);
  });
