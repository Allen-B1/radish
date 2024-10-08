let data = `
<PROVINCES>
		<!-- name, abbreviation, and adjacency data for all provinces -->
		<!-- virtually identical to Judge format -->
		<PROVINCE shortname="swi" fullname="Switzerland">
			<ADJACENCY type="mv" refs="swi" />
		</PROVINCE>
		<PROVINCE shortname="adr" fullname="Adriatic Sea">
			<ADJACENCY type="xc" refs="alb apu ven tri ion" />
		</PROVINCE>
		<PROVINCE shortname="aeg" fullname="Aegean Sea">
			<ADJACENCY type="xc" refs="gre bul-sc con smy eas ion" />
		</PROVINCE>
		<PROVINCE shortname="alb" fullname="Albania">
			<ADJACENCY type="mv" refs="tri gre ser" />
			<ADJACENCY type="xc" refs="adr tri gre ion" />
		</PROVINCE>
		<PROVINCE shortname="ank" fullname="Ankara">
			<ADJACENCY type="mv" refs="arm con smy" />
			<ADJACENCY type="xc" refs="bla arm con" />
		</PROVINCE>
		<PROVINCE shortname="apu" fullname="Apulia">
			<ADJACENCY type="mv" refs="ven nap rom" />
			<ADJACENCY type="xc" refs="ven adr ion nap" />
		</PROVINCE>
		<PROVINCE shortname="arm" fullname="Armenia">
			<ADJACENCY type="mv" refs="smy syr ank sev" />
			<ADJACENCY type="xc" refs="ank sev bla" />
		</PROVINCE>
		<PROVINCE shortname="bal" fullname="Baltic Sea">
			<ADJACENCY type="xc" refs="lvn pru ber kie den swe bot" />
		</PROVINCE>
		<PROVINCE shortname="bar" fullname="Barents Sea">
			<ADJACENCY type="xc" refs="nwg stp-nc nwy" />
		</PROVINCE>
		<PROVINCE shortname="bel" fullname="Belgium">
			<ADJACENCY type="mv" refs="hol pic ruh bur" />
			<ADJACENCY type="xc" refs="eng nth hol pic" />
		</PROVINCE>
		<PROVINCE shortname="ber" fullname="Berlin">
			<ADJACENCY type="mv" refs="kie pru sil mun" />
			<ADJACENCY type="xc" refs="kie bal pru" />
		</PROVINCE>
		<PROVINCE shortname="bla" fullname="Black Sea">
			<ADJACENCY type="xc" refs="rum sev arm ank con bul-ec" />
		</PROVINCE>
		<PROVINCE shortname="boh" fullname="Bohemia">
			<ADJACENCY type="mv" refs="mun sil gal vie tyr" />
		</PROVINCE>
		<PROVINCE shortname="bre" fullname="Brest">
			<UNIQUENAME name="breast"/>
			<ADJACENCY type="mv" refs="pic gas par" />
			<ADJACENCY type="xc" refs="mao eng pic gas" />
		</PROVINCE>
		<PROVINCE shortname="bud" fullname="Budapest">
			<ADJACENCY type="mv" refs="vie gal rum ser tri" />
		</PROVINCE>
		<PROVINCE shortname="bul" fullname="Bulgaria">
			<ADJACENCY type="mv" refs="gre con ser rum" />
			<ADJACENCY type="ec" refs="con bla rum" />
			<ADJACENCY type="sc" refs="gre aeg con" />
		</PROVINCE>
		<PROVINCE shortname="bur" fullname="Burgundy">
			<ADJACENCY type="mv" refs="mar gas par pic bel ruh mun" />
		</PROVINCE>
		<PROVINCE shortname="cly" fullname="Clyde">
			<ADJACENCY type="mv" refs="edi lvp" />
			<ADJACENCY type="xc" refs="edi lvp nao nwg" />
		</PROVINCE>
		<PROVINCE shortname="con" fullname="Constantinople">
			<ADJACENCY type="mv" refs="bul ank smy" />
			<ADJACENCY type="xc" refs="bul-sc bul-ec bla ank smy aeg" />
		</PROVINCE>
		<PROVINCE shortname="den" fullname="Denmark">
			<ADJACENCY type="mv" refs="swe kie" />
			<ADJACENCY type="xc" refs="hel nth swe bal kie ska" />
		</PROVINCE>
		<PROVINCE shortname="eas" fullname="Eastern Mediterranean">
			<UNIQUENAME name="emed" />
			<UNIQUENAME name="eastern med" />
			<UNIQUENAME name="easternmed" />
			<UNIQUENAME name="eastmed" />
			<UNIQUENAME name="ems" />
			<UNIQUENAME name="eme" />
			<ADJACENCY type="xc" refs="syr smy aeg ion" />
		</PROVINCE>
		<PROVINCE shortname="edi" fullname="Edinburgh">
			<ADJACENCY type="mv" refs="lvp yor cly" />
			<ADJACENCY type="xc" refs="nth nwg cly yor" />
		</PROVINCE>
		<PROVINCE shortname="eng" fullname="English Channel">
			<UNIQUENAME name="ech" />
			<UNIQUENAME name="channel" />
			<UNIQUENAME name="man" />
			<ADJACENCY type="xc" refs="mao iri wal lon nth bel pic bre" />
		</PROVINCE>
		<PROVINCE shortname="fin" fullname="Finland">
			<ADJACENCY type="mv" refs="swe stp nwy" />
			<ADJACENCY type="xc" refs="swe stp-sc bot" />
		</PROVINCE>
		<PROVINCE shortname="gal" fullname="Galicia">
			<ADJACENCY type="mv" refs="war ukr rum bud vie boh sil" />
		</PROVINCE>
		<PROVINCE shortname="gas" fullname="Gascony">
			<ADJACENCY type="mv" refs="par bur mar spa bre" />
			<ADJACENCY type="xc" refs="spa-nc mao bre" />
		</PROVINCE>
		<PROVINCE shortname="gre" fullname="Greece">
			<ADJACENCY type="mv" refs="bul alb ser" />
			<ADJACENCY type="xc" refs="bul-sc aeg ion alb" />
		</PROVINCE>
		<PROVINCE shortname="bot" fullname="Gulf of Bothnia">
			<UNIQUENAME name="gob" />
			<UNIQUENAME name="gulfofb" />
			<UNIQUENAME name="bothnia" />
			<UNIQUENAME name="gulf of bot" />
			<ADJACENCY type="xc" refs="swe fin stp-sc lvn bal" />
		</PROVINCE>
		<PROVINCE shortname="lyo" fullname="Gulf of Lyon">
			<UNIQUENAME name="gulf of lyo" />
			<UNIQUENAME name="gol" />
			<UNIQUENAME name="gulfofl" />
			<UNIQUENAME name="lyon" />
			<ADJACENCY type="xc" refs="spa-sc mar pie tus tys wes" />
		</PROVINCE>
		<PROVINCE shortname="hel" fullname="Helgoland Bight">
			<ADJACENCY type="xc" refs="nth den kie hol" />
		</PROVINCE>
		<PROVINCE shortname="hol" fullname="Holland">
			<ADJACENCY type="mv" refs="bel kie ruh" />
			<ADJACENCY type="xc" refs="bel nth hel kie" />
		</PROVINCE>
		<PROVINCE shortname="ion" fullname="Ionian Sea">
			<ADJACENCY type="xc" refs="tun tys nap apu adr alb gre aeg eas" />
		</PROVINCE>
		<PROVINCE shortname="iri" fullname="Irish Sea">
			<UNIQUENAME name="stg" />
			<ADJACENCY type="xc" refs="nao lvp wal eng mao" />
		</PROVINCE>
		<PROVINCE shortname="kie" fullname="Kiel">
			<ADJACENCY type="mv" refs="hol den ber mun ruh" />
			<ADJACENCY type="xc" refs="hol hel den bal ber" />
		</PROVINCE>
		<PROVINCE shortname="lon" fullname="London">
			<ADJACENCY type="mv" refs="yor wal" />
			<ADJACENCY type="xc" refs="yor nth eng wal" />
		</PROVINCE>			
		<PROVINCE shortname="lvn" fullname="Livonia">
			<UNIQUENAME name="lvo" />
			<UNIQUENAME name="lva" />
			<ADJACENCY type="mv" refs="pru stp mos war" />
			<ADJACENCY type="xc" refs="pru bal bot stp-sc" />
		</PROVINCE>
		<PROVINCE shortname="lvp" fullname="Liverpool">
			<UNIQUENAME name="lpl" />
			<ADJACENCY type="mv" refs="wal edi yor cly" />
			<ADJACENCY type="xc" refs="wal iri nao cly" />
		</PROVINCE>
		
		<PROVINCE shortname="mar" fullname="Marseilles">
			<ADJACENCY type="mv" refs="spa pie gas bur" />
			<ADJACENCY type="xc" refs="spa-sc lyo pie" />
		</PROVINCE>
		<PROVINCE shortname="mao" fullname="Mid-Atlantic Ocean">
			<UNIQUENAME name="mid atlantic ocean" />  <!-- note: no hyphen! -->
			<UNIQUENAME name="mid-atlantic" />
			<UNIQUENAME name="mid atlantic" />
			<UNIQUENAME name="midatlanticocean" />
			<UNIQUENAME name="mid" />
			<UNIQUENAME name="mat" />
			<ADJACENCY type="xc" refs="nao iri eng bre gas spa-nc por spa-sc naf wes" />
		</PROVINCE>
		<PROVINCE shortname="mos" fullname="Moscow">
			<ADJACENCY type="mv" refs="stp lvn war ukr sev" />
		</PROVINCE>
		<PROVINCE shortname="mun" fullname="Munich">
			<ADJACENCY type="mv" refs="bur ruh kie ber sil boh tyr" />
		</PROVINCE>
		<PROVINCE shortname="naf" fullname="North Africa">
			<UNIQUENAME name="nora" />
			<UNIQUENAME name="northafrica" />
			<ADJACENCY type="mv" refs="tun" />
			<ADJACENCY type="xc" refs="mao wes tun" />
		</PROVINCE>
		<PROVINCE shortname="nap" fullname="Naples">
			<ADJACENCY type="mv" refs="rom apu" />
			<ADJACENCY type="xc" refs="rom tys ion apu" />
		</PROVINCE>
		<PROVINCE shortname="nao" fullname="North Atlantic Ocean">
			<UNIQUENAME name="northatlantic" />
			<UNIQUENAME name="north atlantic" />
			<UNIQUENAME name="nat" />
			<ADJACENCY type="xc" refs="nwg lvp iri mao cly" />
		</PROVINCE>
		<PROVINCE shortname="nth" fullname="North Sea">
			<UNIQUENAME name="norsea" />
			<UNIQUENAME name="northsea" />
			<UNIQUENAME name="nts" />
			<UNIQUENAME name="nrd" />
			<ADJACENCY type="xc" refs="yor edi nwg nwy ska den hel hol bel eng lon" />
		</PROVINCE>
		<PROVINCE shortname="nwy" fullname="Norway">
			<UNIQUENAME name="nwy" />
			<UNIQUENAME name="nge" />
			<ADJACENCY type="mv" refs="fin stp swe" />
			<ADJACENCY type="xc" refs="ska nth nwg bar stp-nc swe" />
		</PROVINCE>
		<PROVINCE shortname="nwg" fullname="Norwegian Sea">
			<UNIQUENAME name="norwegiansea" />
			<UNIQUENAME name="norwsea" />
			<UNIQUENAME name="nrg" />
			<UNIQUENAME name="gro" />
			<ADJACENCY type="xc" refs="nao bar nwy nth cly edi" />
		</PROVINCE>
		<PROVINCE shortname="par" fullname="Paris">
			<ADJACENCY type="mv" refs="bre pic bur gas" />
		</PROVINCE>
		<PROVINCE shortname="pic" fullname="Picardy">
			<ADJACENCY type="mv" refs="bur par bre bel" />
			<ADJACENCY type="xc" refs="bre eng bel" />
		</PROVINCE>
		<PROVINCE shortname="pie" fullname="Piedmont">
			<ADJACENCY type="mv" refs="mar tus ven tyr" />
			<ADJACENCY type="xc" refs="mar lyo tus" />
		</PROVINCE>
		<PROVINCE shortname="por" fullname="Portugal">
			<ADJACENCY type="mv" refs="spa" />
			<ADJACENCY type="xc" refs="mao spa-nc spa-sc" />
		</PROVINCE>
		<PROVINCE shortname="pru" fullname="Prussia">
			<ADJACENCY type="mv" refs="war sil ber lvn" />
			<ADJACENCY type="xc" refs="ber bal lvn" />
		</PROVINCE>
		<PROVINCE shortname="rom" fullname="Rome">
			<ADJACENCY type="mv" refs="tus nap ven apu" />
			<ADJACENCY type="xc" refs="tus tys nap" />
		</PROVINCE>
		<PROVINCE shortname="ruh" fullname="Ruhr">
			<UNIQUENAME name="rhe" />
			<ADJACENCY type="mv" refs="bur bel hol kie mun" />
		</PROVINCE>
		<PROVINCE shortname="rum" fullname="Rumania">
			<ADJACENCY type="mv" refs="ser bud gal ukr sev bul" />
			<ADJACENCY type="xc" refs="sev bla bul-ec" />
		</PROVINCE>
		<PROVINCE shortname="ser" fullname="Serbia">
			<ADJACENCY type="mv" refs="tri bud rum bul gre alb" />
		</PROVINCE>
		<PROVINCE shortname="sev" fullname="Sevastopol">
			<UNIQUENAME name="sevastapol" />
			<ADJACENCY type="mv" refs="ukr mos rum arm" />
			<ADJACENCY type="xc" refs="rum bla arm" />
		</PROVINCE>
		<PROVINCE shortname="sil" fullname="Silesia">
			<ADJACENCY type="mv" refs="mun ber pru war gal boh" />
		</PROVINCE>
		<PROVINCE shortname="ska" fullname="Skagerrak">
			<UNIQUENAME name="skaggerak" />
			<ADJACENCY type="xc" refs="nth nwy den swe" />
		</PROVINCE>
		<PROVINCE shortname="smy" fullname="Smyrna">
			<ADJACENCY type="mv" refs="syr con ank arm" />
			<ADJACENCY type="xc" refs="syr eas aeg con" />
		</PROVINCE>
		<PROVINCE shortname="spa" fullname="Spain">
			<UNIQUENAME name="esp" />
			<ADJACENCY type="mv" refs="gas por mar" />
			<ADJACENCY type="nc" refs="gas mao por" />
			<ADJACENCY type="sc" refs="por wes lyo mar mao" />
		</PROVINCE>
		<PROVINCE shortname="stp" fullname="St. Petersburg">
			<UNIQUENAME name="st petersburg" />  <!-- note: no period after 'st' -->
			<UNIQUENAME name="stpetersburg" />
			<UNIQUENAME name="st petersberg" />
			<UNIQUENAME name="st. petersberg" />
			<UNIQUENAME name="stpetersberg" />
			<ADJACENCY type="mv" refs="fin lvn nwy mos" />
			<ADJACENCY type="nc" refs="bar nwy" />
			<ADJACENCY type="sc" refs="fin lvn bot" />
		</PROVINCE>
		<PROVINCE shortname="swe" fullname="Sweden">
			<ADJACENCY type="mv" refs="fin den nwy" />
			<ADJACENCY type="xc" refs="fin bot bal den ska nwy" />
		</PROVINCE>
		<PROVINCE shortname="syr" fullname="Syria">
			<ADJACENCY type="mv" refs="smy arm" />
			<ADJACENCY type="xc" refs="eas smy" />
		</PROVINCE>
		<PROVINCE shortname="tri" fullname="Trieste">
			<ADJACENCY type="mv" refs="tyr vie bud ser alb ven" />
			<ADJACENCY type="xc" refs="alb adr ven" />
		</PROVINCE>
		<PROVINCE shortname="tun" fullname="Tunis">
			<ADJACENCY type="mv" refs="naf" />
			<ADJACENCY type="xc" refs="naf wes tys ion" />
		</PROVINCE>
		<PROVINCE shortname="tus" fullname="Tuscany">
			<ADJACENCY type="mv" refs="rom pie ven" />
			<ADJACENCY type="xc" refs="rom tys lyo pie" />
		</PROVINCE>
		<PROVINCE shortname="tyr" fullname="Tyrolia">
			<UNIQUENAME name="tyl" />
			<UNIQUENAME name="trl" />
			<UNIQUENAME name="alp" />
			<ADJACENCY type="mv" refs="mun boh vie tri ven pie" />
		</PROVINCE>
		<PROVINCE shortname="tys" fullname="Tyrrhenian Sea">
			<UNIQUENAME name="tyrhennian sea" />
			<UNIQUENAME name="tyrrhenian sea" />
			<UNIQUENAME name="tyrhenian sea" />
			<UNIQUENAME name="tyn" />
			<UNIQUENAME name="tyh" />
			<ADJACENCY type="xc" refs="wes lyo tus rom nap ion tun" />
		</PROVINCE>
		<PROVINCE shortname="ukr" fullname="Ukraine">
			<ADJACENCY type="mv" refs="rum gal war mos sev" />
		</PROVINCE>
		<PROVINCE shortname="ven" fullname="Venice">
			<ADJACENCY type="mv" refs="tyr tus rom pie apu tri" />
			<ADJACENCY type="xc" refs="apu adr tri" />
		</PROVINCE>
		<PROVINCE shortname="vie" fullname="Vienna">
			<ADJACENCY type="mv" refs="tyr boh gal bud tri" />
		</PROVINCE>
		<PROVINCE shortname="wal" fullname="Wales">
			<ADJACENCY type="mv" refs="lvp lon yor" />
			<ADJACENCY type="xc" refs="lvp iri eng lon" />
		</PROVINCE>
		<PROVINCE shortname="war" fullname="Warsaw">
			<UNIQUENAME name="var" />
			<ADJACENCY type="mv" refs="sil pru lvn mos ukr gal" />
		</PROVINCE>
		<PROVINCE shortname="wes" fullname="Western Mediterranean">
			<UNIQUENAME name="western med" />
			<UNIQUENAME name="westernmed" />
			<UNIQUENAME name="wmed" />
			<UNIQUENAME name="westmed" />
			<UNIQUENAME name="wms" />
			<UNIQUENAME name="wme" />
			<ADJACENCY type="xc" refs="mao spa-sc lyo tys tun naf" />
		</PROVINCE>
		<PROVINCE shortname="yor" fullname="Yorkshire">
			<UNIQUENAME name="yonkers" />
			<ADJACENCY type="mv" refs="edi lon lvp wal" />
			<ADJACENCY type="xc" refs="edi nth lon" />
		</PROVINCE>
</PROVINCES>
`;


let xml = new DOMParser().parseFromString(data, "text/xml");
let json = {
	provinces: {},
  fleet_adj: [],
  army_adj: []
};
for (let child of xml.children[0].children) {
  	let abbr = child.getAttribute("shortname");
    let name = child.getAttribute("fullname");
	let coasts = [];
    let is_land = false;
	for (let adj of child.children) {
    	if (adj.tagName.toLowerCase() != "adjacency") continue;
        
        let coast = adj.getAttribute("type");
        coast = coast == "xc" ? "": coast;
        let targets = adj.getAttribute("refs").split(" ");
        switch (coast) {
   			case "mv":
        		is_land = true;
        		Array.prototype.push.apply(json.army_adj, targets.map(x => [abbr, x]));
            break;
        default:
        		Array.prototype.push.apply(json.fleet_adj, targets.map(x => [[abbr, coast], [x.split("-")[0], x.split("-")[1] || ""]]));
        		Array.prototype.push.apply(json.fleet_adj, targets.map(x => [[abbr, coast], [x.split("-")[0], ""]]));
        		Array.prototype.push.apply(json.fleet_adj, targets.map(x => [[abbr, ""], [x.split("-")[0], x.split("-")[1] || ""]]));
        		Array.prototype.push.apply(json.fleet_adj, targets.map(x => [[abbr, ""], [x.split("-")[0], ""]]));
        }
        
        if (coast && coast != "mv") {
        	coasts.push(coast);
        }
    }

    json.provinces[abbr] = {
    	name: name,
        coasts: coasts,
        is_sea: !is_land
    };
}

json.fleet_adj = [...new Set(json.fleet_adj)];
json.fleet_adj.sort();

document.write(JSON.stringify(json));