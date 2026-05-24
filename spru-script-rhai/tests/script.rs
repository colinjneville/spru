use spru::item::IdT;
use spru_util::cloned;
use tagset::tagset;

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[spru_script::script(include = [Impl])]
struct S {
    #[get]
    #[set]
    a: i64,

    #[get]
    #[set]
    b: Option<IdT<S>>,
}

#[spru_script::script(partial = Impl)]
impl S {
    #[create]
    fn create(val: i64, dummy: i64) -> cloned::Create<S> {
        cloned::create(Self {
            a: val,
            b: None,
        })
    }

    #[function]
    fn f() -> i64 {
        9i64
    }
}

#[tagset(impl tagset::proxy::serde::Serialize)]
#[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
#[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
#[tagset(impl spru::State)]
#[tagset(impl<Action, Registry> spru_script::Scriptable<Action, Registry>)]
#[tagset(derive(Debug))]
#[tagset(S)]
struct MyState;

#[tagset(impl spru::Action {
    type State = MyState;
})]
#[tagset(impl tagset::proxy::serde::Serialize)]
#[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
#[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
#[tagset(derive(Debug, Clone))]
#[tagset(include(cloned::Actions<S>))]
struct MyAction;

#[test]
fn script() {
    use spru_script::Language as _;
    let storage = spru_util::storage::Standalone::<MyState>::new();
    let rhai = spru_script_rhai::Rhai::<MyState, MyAction>::default();
    let mut test_interactor = spru::interactor::test_util::TestInteractor::new(storage);
    let mut interactor = test_interactor.interactor::<MyAction, _, ()>(&());
    let s = interactor
        .create(cloned::create(S { a: 3i64, b: None, }));
    let root = s.id();

    interactor.flush().unwrap();

    let mut interactor = test_interactor.interactor::<MyAction, _, ()>(&root);

    let script = r#"
    let b = type["S"].create(7, 5);
    context.root.b = b.some;
    // let a = S.f();
    // a
    context.root.b.a
    "#;

    let a: i64 = rhai.exec(&mut interactor, script, rhai::Dynamic::UNIT)
        .unwrap();

    assert_eq!(a, 7i64);
}